//! The credits of a document, derived from its objects (ADR 0085 decision 7, `specs/0138`
//! criterion 4): every object whose Came from record carries a credit text lists it; nothing is
//! stored, so deleting the object takes its credit with it.

use loro::TreeParentId;

use crate::came_from;
use crate::document::{Document, OBJECTS_TREE};

/// One line of the File tab's Credits block.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Credit {
    /// The credit text as the catalogue states it ("Box by Jane Doe, CC BY 4.0").
    pub text: String,
    /// The licence's name, shown as a link.
    pub licence: String,
    /// The licence text's address.
    pub licence_url: String,
    /// The address the item came from, for the tooltip.
    pub address: String,
}

impl Document {
    /// The credits of the objects in the tree, in tree order, identical ones once.
    #[must_use]
    pub fn credits(&self) -> Vec<Credit> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut found: Vec<Credit> = Vec::new();
        crate::fragment::walk(&tree, TreeParentId::Root, &mut |node| {
            let credit = tree
                .get_meta(node)
                .ok()
                .and_then(|meta| came_from::read(&meta))
                .and_then(|record| record.online)
                .and_then(|online| {
                    let text = online.credit?;
                    (!text.trim().is_empty()).then_some(Credit {
                        text,
                        licence: online.licence,
                        licence_url: online.licence_url,
                        address: online.address,
                    })
                });
            if let Some(credit) = credit
                && !found
                    .iter()
                    .any(|c| c.text == credit.text && c.licence == credit.licence)
            {
                found.push(credit);
            }
            true
        });
        found
    }
}
