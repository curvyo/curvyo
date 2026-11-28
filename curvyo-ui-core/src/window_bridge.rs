//! The message shape between the two windows of one project (ADR 0096 decision 3, amendment A5):
//! one bundle of pictures, updates, selection and controls per direction per frame, as bytes.
//!
//! Nothing is sealed: the bytes stay inside the app process and never leave the machine. Both
//! pages run the same build, but the version byte is still checked, so a stale page never reads
//! a bundle it does not understand.

use curvyo_document_core::{AssetHash, NodeId};

/// The bundle version this build writes and reads.
pub const BRIDGE_VERSION: u8 = 1;
const KIND_BUNDLE: u8 = 1;
const KIND_PEER_GONE: u8 = 2;
/// The fixed message the hub sends the main page when the pen window is gone (A5).
pub const PEER_GONE: [u8; 2] = [BRIDGE_VERSION, KIND_PEER_GONE];

const TAG_ASSET: u8 = 1;
const TAG_UPDATES: u8 = 2;
const TAG_SELECTION: u8 = 3;
const TAG_PROJECT: u8 = 10;
const TAG_NEED_PROJECT: u8 = 11;
const TAG_NEED_ASSETS: u8 = 12;
const TAG_HOLD: u8 = 13;
const TAG_RELEASE: u8 = 14;
const TAG_SAVE: u8 = 15;
const TAG_SHARED: u8 = 16;
const TAG_PEER: u8 = 17;
const TAG_VIEW: u8 = 18;
/// The bytes of a `View` item's body.
const VIEW_LEN: usize = 4 + 4 + 1 + 8 * 7;
/// `needs` absent in an updates item.
const NO_NEEDS: u32 = u32::MAX;

/// Why a full project is sent (A6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectReason {
    /// The pen window's first load.
    Start,
    /// A reload after a refused import (criterion 18): tool and view stay.
    Reload,
    /// The main window opened another project or New (criterion 12): the view is fitted.
    Replaced,
}

/// A full project: the packed document and what the pen session needs beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMessage {
    /// Why it is sent.
    pub reason: ProjectReason,
    /// Whether the main document refuses editable curves and width profiles (A12).
    pub shared: bool,
    /// The main window's active page id, for the pen window's first fit (criterion 22).
    pub page: String,
    /// The bytes of `Session::pack`.
    pub bytes: Vec<u8>,
}

/// The document updates of one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Updates {
    /// The bytes of `Document::export_updates`.
    pub bytes: Vec<u8>,
    /// The sender's referenced asset hashes, sent only when that set changed.
    pub needs: Option<Vec<AssetHash>>,
}

/// One window's view for the other (2b, A5): what Follow view and the frame in the main window
/// read. View state only, never document content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewMessage {
    /// Views this sender has sent, this one included.
    pub count: u32,
    /// The other side's `count` this sender had applied or seen when it sent this one.
    pub seen: u32,
    /// Whether the pen window's Follow view is on.
    pub follow: bool,
    /// Whether this window's own pan or zoom moved the view (only such a view is followed).
    pub moved: bool,
    /// The zoom factor, `1.0` for 100 %.
    pub zoom: f64,
    /// The document point at the canvas's centre, mm.
    pub centre: (f64, f64),
    /// The visible rectangle in document mm: left, top, right, bottom.
    pub visible: [f64; 4],
}

/// A control item of a bundle.
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    /// A full project (main to pen).
    Project(ProjectMessage),
    /// The pen asks for a full project (pen to main).
    NeedProject,
    /// The receiver lacks these pictures for an update that waits.
    NeedAssets(Vec<AssetHash>),
    /// A fate-deciding dialog of the main window is open (criterion 13).
    Hold,
    /// It closed.
    Release,
    /// Ctrl+S in the pen window (criterion 9).
    Save,
    /// The main document's blocked state for extended path edits changed (A12).
    Shared(bool),
    /// The sender's Loro peer id, sent after each project it opened (History names, A7).
    Peer(u64),
    /// The sender's view (2b).
    View(ViewMessage),
}

/// One bundle: the sender's sequence number within its generation and the items in wire order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bundle {
    /// The project generation the sender is on; the main window raises it with every project.
    pub generation: u32,
    /// +1 per bundle within a generation, from 0.
    pub seq: u64,
    /// Pictures the other side lacks, before the updates that reference them.
    pub assets: Vec<(AssetHash, Vec<u8>)>,
    /// The frame's document updates.
    pub updates: Option<Updates>,
    /// The sender's selection, when it changed.
    pub selection: Option<Vec<NodeId>>,
    /// Controls, after everything else.
    pub controls: Vec<Control>,
}

impl Bundle {
    /// Whether the bundle carries nothing (then it is not sent).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
            && self.updates.is_none()
            && self.selection.is_none()
            && self.controls.is_empty()
    }
}

/// What arrived on the bridge.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// A bundle of the other window.
    Bundle(Bundle),
    /// The hub's notice that the pen window is gone.
    PeerGone,
}

/// Why bytes are not a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BridgeError {
    /// Another bundle version or kind.
    #[error("another bridge version")]
    Version,
    /// The bytes end inside an item.
    #[error("the bundle is cut short")]
    Truncated,
    /// An item this build does not know, or one out of order.
    #[error("unknown or misplaced item {0}")]
    Item(u8),
    /// A selection, hash or page that does not read.
    #[error("an item does not read")]
    Malformed,
}

/// The bytes of `bundle`.
#[must_use]
pub fn encode(bundle: &Bundle) -> Vec<u8> {
    let mut out = vec![BRIDGE_VERSION, KIND_BUNDLE];
    out.extend_from_slice(&bundle.generation.to_le_bytes());
    out.extend_from_slice(&bundle.seq.to_le_bytes());
    for (hash, bytes) in &bundle.assets {
        let mut body = hash.to_bytes().to_vec();
        body.extend_from_slice(bytes);
        item(&mut out, TAG_ASSET, &body);
    }
    if let Some(updates) = &bundle.updates {
        let mut body = Vec::with_capacity(updates.bytes.len() + 4);
        match &updates.needs {
            Some(needs) => {
                body.extend_from_slice(&len32(needs.len()).to_le_bytes());
                body.extend(hashes(needs));
            }
            None => body.extend_from_slice(&NO_NEEDS.to_le_bytes()),
        }
        body.extend_from_slice(&updates.bytes);
        item(&mut out, TAG_UPDATES, &body);
    }
    if let Some(selection) = &bundle.selection {
        // invariant: a list of node ids always serialises.
        let body = serde_json::to_vec(selection).unwrap_or_default();
        item(&mut out, TAG_SELECTION, &body);
    }
    for control in &bundle.controls {
        encode_control(&mut out, control);
    }
    out
}

fn encode_control(out: &mut Vec<u8>, control: &Control) {
    match control {
        Control::Project(project) => {
            let page = project.page.as_bytes();
            let mut body = vec![reason_byte(project.reason), u8::from(project.shared)];
            body.extend_from_slice(&len32(page.len()).to_le_bytes());
            body.extend_from_slice(page);
            body.extend_from_slice(&project.bytes);
            item(out, TAG_PROJECT, &body);
        }
        Control::NeedProject => item(out, TAG_NEED_PROJECT, &[]),
        Control::NeedAssets(needed) => item(out, TAG_NEED_ASSETS, &hashes(needed)),
        Control::Hold => item(out, TAG_HOLD, &[]),
        Control::Release => item(out, TAG_RELEASE, &[]),
        Control::Save => item(out, TAG_SAVE, &[]),
        Control::Shared(on) => item(out, TAG_SHARED, &[u8::from(*on)]),
        Control::Peer(peer) => item(out, TAG_PEER, &peer.to_le_bytes()),
        Control::View(view) => item(out, TAG_VIEW, &view_body(view)),
    }
}

fn view_body(view: &ViewMessage) -> Vec<u8> {
    let mut body = Vec::with_capacity(VIEW_LEN);
    body.extend_from_slice(&view.count.to_le_bytes());
    body.extend_from_slice(&view.seen.to_le_bytes());
    body.push(u8::from(view.follow) | (u8::from(view.moved) << 1));
    let numbers = [view.zoom, view.centre.0, view.centre.1];
    for number in numbers.iter().chain(&view.visible) {
        body.extend_from_slice(&number.to_le_bytes());
    }
    body
}

fn read_view(mut reader: Reader<'_>) -> Result<ViewMessage, BridgeError> {
    if reader.0.len() != VIEW_LEN {
        return Err(BridgeError::Malformed);
    }
    let count = u32::from_le_bytes(reader.array()?);
    let seen = u32::from_le_bytes(reader.array()?);
    let [flags] = reader.array()?;
    let mut numbers = [0.0f64; 7];
    for number in &mut numbers {
        *number = f64::from_le_bytes(reader.array()?);
    }
    if numbers.iter().any(|n| !n.is_finite()) {
        return Err(BridgeError::Malformed);
    }
    let [zoom, cx, cy, left, top, right, bottom] = numbers;
    Ok(ViewMessage {
        count,
        seen,
        follow: flags & 1 == 1,
        moved: flags & 2 == 2,
        zoom,
        centre: (cx, cy),
        visible: [left, top, right, bottom],
    })
}

/// The message in `bytes`.
///
/// # Errors
/// [`BridgeError`] for another version, a cut or unknown item, or items out of wire order.
pub fn decode(bytes: &[u8]) -> Result<Message, BridgeError> {
    match bytes {
        [BRIDGE_VERSION, KIND_PEER_GONE] => return Ok(Message::PeerGone),
        [BRIDGE_VERSION, KIND_BUNDLE, ..] => {}
        _ => return Err(BridgeError::Version),
    }
    let mut reader = Reader(bytes.get(2..).ok_or(BridgeError::Truncated)?);
    let generation = u32::from_le_bytes(reader.array()?);
    let seq = u64::from_le_bytes(reader.array()?);
    let mut bundle = Bundle {
        generation,
        seq,
        ..Bundle::default()
    };
    let mut last = 0u8;
    while !reader.0.is_empty() {
        let tag = reader.take(1)?[0];
        let len = u32::from_le_bytes(reader.array()?) as usize;
        let body = reader.take(len)?;
        // Assets repeat; everything else comes once and in tag order, controls after it.
        let in_order = tag > last || (tag == TAG_ASSET && last == TAG_ASSET) || tag >= TAG_PROJECT;
        if !in_order {
            return Err(BridgeError::Item(tag));
        }
        last = last.max(tag);
        decode_item(&mut bundle, tag, body)?;
    }
    Ok(Message::Bundle(bundle))
}

fn decode_item(bundle: &mut Bundle, tag: u8, body: &[u8]) -> Result<(), BridgeError> {
    let mut reader = Reader(body);
    match tag {
        TAG_ASSET => {
            let hash = AssetHash::from_bytes(&reader.array()?);
            bundle.assets.push((hash, reader.0.to_vec()));
        }
        TAG_UPDATES => {
            let count = u32::from_le_bytes(reader.array()?);
            let needs = if count == NO_NEEDS {
                None
            } else {
                Some(read_hashes(reader.take(count as usize * 32)?)?)
            };
            bundle.updates = Some(Updates {
                bytes: reader.0.to_vec(),
                needs,
            });
        }
        TAG_SELECTION => {
            bundle.selection =
                Some(serde_json::from_slice(body).map_err(|_| BridgeError::Malformed)?);
        }
        TAG_PROJECT => bundle
            .controls
            .push(Control::Project(read_project(reader)?)),
        TAG_NEED_PROJECT => bundle.controls.push(Control::NeedProject),
        TAG_NEED_ASSETS => bundle
            .controls
            .push(Control::NeedAssets(read_hashes(body)?)),
        TAG_HOLD => bundle.controls.push(Control::Hold),
        TAG_RELEASE => bundle.controls.push(Control::Release),
        TAG_SAVE => bundle.controls.push(Control::Save),
        TAG_SHARED => bundle
            .controls
            .push(Control::Shared(reader.array::<1>()? == [1])),
        TAG_PEER => bundle
            .controls
            .push(Control::Peer(u64::from_le_bytes(reader.array()?))),
        TAG_VIEW => bundle.controls.push(Control::View(read_view(reader)?)),
        other => return Err(BridgeError::Item(other)),
    }
    Ok(())
}

fn read_project(mut reader: Reader<'_>) -> Result<ProjectMessage, BridgeError> {
    let [reason, shared] = reader.array()?;
    let reason = match reason {
        0 => ProjectReason::Start,
        1 => ProjectReason::Reload,
        2 => ProjectReason::Replaced,
        _ => return Err(BridgeError::Malformed),
    };
    let page_len = u32::from_le_bytes(reader.array()?) as usize;
    let page = std::str::from_utf8(reader.take(page_len)?)
        .map_err(|_| BridgeError::Malformed)?
        .to_owned();
    Ok(ProjectMessage {
        reason,
        shared: shared == 1,
        page,
        bytes: reader.0.to_vec(),
    })
}

const fn reason_byte(reason: ProjectReason) -> u8 {
    match reason {
        ProjectReason::Start => 0,
        ProjectReason::Reload => 1,
        ProjectReason::Replaced => 2,
    }
}

fn item(out: &mut Vec<u8>, tag: u8, body: &[u8]) {
    out.push(tag);
    out.extend_from_slice(&len32(body.len()).to_le_bytes());
    out.extend_from_slice(body);
}

/// A length as the wire's `u32`; nothing in one process comes near 4 GiB.
fn len32(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX - 1)
}

fn hashes(list: &[AssetHash]) -> Vec<u8> {
    list.iter().flat_map(AssetHash::to_bytes).collect()
}

fn read_hashes(bytes: &[u8]) -> Result<Vec<AssetHash>, BridgeError> {
    let (chunks, rest) = bytes.as_chunks::<32>();
    if !rest.is_empty() {
        return Err(BridgeError::Malformed);
    }
    Ok(chunks.iter().map(AssetHash::from_bytes).collect())
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], BridgeError> {
        if self.0.len() < len {
            return Err(BridgeError::Truncated);
        }
        let (head, tail) = self.0.split_at(len);
        self.0 = tail;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], BridgeError> {
        let bytes = self.take(N)?;
        let mut out = [0; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> Bundle {
        let picture = vec![7u8; 40];
        Bundle {
            generation: 3,
            seq: 42,
            assets: vec![(AssetHash::of(&picture), picture.clone())],
            updates: Some(Updates {
                bytes: vec![1, 2, 3],
                needs: Some(vec![AssetHash::of(&picture)]),
            }),
            selection: Some(Vec::new()),
            controls: vec![
                Control::Project(ProjectMessage {
                    reason: ProjectReason::Replaced,
                    shared: true,
                    page: "page-1".to_owned(),
                    bytes: vec![9; 10],
                }),
                Control::NeedProject,
                Control::NeedAssets(vec![AssetHash::of(b"x")]),
                Control::Hold,
                Control::Release,
                Control::Save,
                Control::Shared(false),
                Control::Peer(u64::MAX),
                Control::View(ViewMessage {
                    count: 7,
                    seen: 3,
                    follow: true,
                    moved: false,
                    zoom: 2.5,
                    centre: (105.25, -3.5),
                    visible: [10.0, 20.0, 200.5, 120.125],
                }),
            ],
        }
    }

    #[test]
    fn a_bundle_reads_back_unchanged() {
        let bundle = full();
        assert_eq!(decode(&encode(&bundle)), Ok(Message::Bundle(bundle)));
        let bare = Bundle {
            updates: Some(Updates {
                bytes: Vec::new(),
                needs: None,
            }),
            ..Bundle::default()
        };
        assert_eq!(decode(&encode(&bare)), Ok(Message::Bundle(bare)));
    }

    #[test]
    fn an_empty_bundle_is_empty_and_peer_gone_is_its_own_message() {
        assert!(Bundle::default().is_empty());
        assert!(!full().is_empty());
        assert_eq!(decode(&PEER_GONE), Ok(Message::PeerGone));
    }

    #[test]
    fn another_version_a_cut_or_an_unknown_item_is_refused() {
        let mut bytes = encode(&full());
        bytes[0] = 2;
        assert_eq!(decode(&bytes), Err(BridgeError::Version));
        let bytes = encode(&full());
        assert_eq!(
            decode(&bytes[..bytes.len() - 1]),
            Err(BridgeError::Truncated)
        );
        let mut unknown = encode(&Bundle::default());
        item(&mut unknown, 99, &[]);
        assert_eq!(decode(&unknown), Err(BridgeError::Item(99)));
        assert_eq!(decode(&[]), Err(BridgeError::Version));
    }

    #[test]
    fn a_view_of_another_length_or_not_a_number_is_refused() {
        let mut short = encode(&Bundle::default());
        item(&mut short, TAG_VIEW, &[0; VIEW_LEN - 1]);
        assert_eq!(decode(&short), Err(BridgeError::Malformed));
        let mut body = view_body(&ViewMessage {
            count: 1,
            seen: 0,
            follow: false,
            moved: true,
            zoom: 1.0,
            centre: (0.0, 0.0),
            visible: [0.0; 4],
        });
        body[9..17].copy_from_slice(&f64::NAN.to_le_bytes());
        let mut nan = encode(&Bundle::default());
        item(&mut nan, TAG_VIEW, &body);
        assert_eq!(decode(&nan), Err(BridgeError::Malformed));
    }

    #[test]
    fn items_out_of_wire_order_are_refused() {
        let mut bytes = encode(&Bundle::default());
        item(&mut bytes, TAG_SELECTION, b"[]");
        item(&mut bytes, TAG_UPDATES, &NO_NEEDS.to_le_bytes());
        assert_eq!(decode(&bytes), Err(BridgeError::Item(TAG_UPDATES)));
    }
}
