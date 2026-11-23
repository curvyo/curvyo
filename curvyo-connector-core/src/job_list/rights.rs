//! Who may do what to a job of the list (`specs/0132` criterion 5, Frage 257 A): one pure table.

use crate::admin::Role;
use crate::machine::Peer;
use crate::message::ErrorCode;

/// What a client does to a job of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Change its place.
    Move,
    /// Take it out.
    Remove,
    /// Set or clear "Run next automatically".
    SetRunNext,
}

/// Whether `peer` may do `action` to a job sent by `sent_by`. `foreground` connectors have no
/// owners: every paired client acts as an owner. An interrupted job is removed by an owner only.
///
/// # Errors
/// [`ErrorCode::OwnerRequired`].
pub fn may(
    action: Action,
    peer: &Peer,
    sent_by: &str,
    interrupted: bool,
    foreground: bool,
) -> Result<(), ErrorCode> {
    if foreground || peer.role == Role::Owner {
        return Ok(());
    }
    let own = peer.name == sent_by;
    match action {
        Action::Remove | Action::SetRunNext if own && !interrupted => Ok(()),
        _ => Err(ErrorCode::OwnerRequired),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_whole_table() {
        let owner = Peer::new("Laptop", Role::Owner);
        let member = Peer::new("Phone", Role::Member);
        let rows = [
            ("Move", Action::Move),
            ("Remove", Action::Remove),
            ("SetRunNext", Action::SetRunNext),
        ];
        // Columns: owner on any, member on own, member on other, member on own interrupted,
        // foreground member on other.
        let mut table = Vec::new();
        for (name, action) in rows {
            let cell = |r: Result<(), ErrorCode>| match r {
                Ok(()) => "yes".to_owned(),
                Err(code) => format!("{code:?}"),
            };
            let cells = [
                cell(may(action, &owner, "Phone", false, false)),
                cell(may(action, &member, "Phone", false, false)),
                cell(may(action, &member, "Tablet", false, false)),
                cell(may(action, &member, "Phone", true, false)),
                cell(may(action, &member, "Tablet", false, true)),
            ];
            table.push(format!("{name}: {}", cells.join(" ")));
        }
        assert_eq!(
            table,
            [
                "Move: yes OwnerRequired OwnerRequired OwnerRequired yes",
                "Remove: yes yes OwnerRequired OwnerRequired yes",
                "SetRunNext: yes yes OwnerRequired OwnerRequired yes",
            ]
        );
    }
}
