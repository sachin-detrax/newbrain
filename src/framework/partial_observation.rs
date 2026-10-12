//! Reusable explicit partial-observation seam.
//!
//! `Unavailable` is a third state, not an atom value.  The representation does
//! not implement `Eq` over unknown slots as if they were unequal observations;
//! callers receive `None` when a comparison cannot be established.

use super::opaque_protocol::{AtomId, ObservationPacket, ProtocolError, OBSERVATION_PACKET_LEN};

#[derive(Clone, Copy, Debug)]
pub enum PartialAtom {
    Available(AtomId),
    Unavailable,
}

#[derive(Clone, Copy, Debug)]
pub struct PartialObservation {
    atoms: [PartialAtom; OBSERVATION_PACKET_LEN],
}

impl PartialObservation {
    pub const fn new(atoms: [PartialAtom; OBSERVATION_PACKET_LEN]) -> Self {
        Self { atoms }
    }

    pub const fn from_packet(packet: ObservationPacket) -> Self {
        let atoms = packet.atoms();
        Self::new([
            PartialAtom::Available(atoms[0]),
            PartialAtom::Available(atoms[1]),
            PartialAtom::Available(atoms[2]),
        ])
    }

    pub const fn atom(self, index: usize) -> Option<PartialAtom> {
        if index < OBSERVATION_PACKET_LEN {
            Some(self.atoms[index])
        } else {
            None
        }
    }

    pub const fn atoms(self) -> [PartialAtom; OBSERVATION_PACKET_LEN] {
        self.atoms
    }

    /// Returns `None` when either side is unavailable.  In particular,
    /// `None` is not a false/negative equality result.
    pub fn known_equal(self, left: usize, right: usize) -> Option<bool> {
        if left >= OBSERVATION_PACKET_LEN || right >= OBSERVATION_PACKET_LEN {
            return None;
        }
        match (self.atoms[left], self.atoms[right]) {
            (PartialAtom::Available(left), PartialAtom::Available(right)) => Some(left == right),
            _ => None,
        }
    }

    /// Evidence for an unchanged position requires both observations. Two
    /// unavailable slots are not evidence of an unchanged position.
    pub fn known_unchanged(self, after: Self, index: usize) -> Option<bool> {
        match (self.atom(index)?, after.atom(index)?) {
            (PartialAtom::Available(before), PartialAtom::Available(after)) => {
                Some(before == after)
            }
            _ => None,
        }
    }

    pub fn try_complete(self) -> Result<ObservationPacket, ProtocolError> {
        let mut atoms = [AtomId::new(0); OBSERVATION_PACKET_LEN];
        for (index, atom) in self.atoms.into_iter().enumerate() {
            atoms[index] = match atom {
                PartialAtom::Available(atom) => atom,
                PartialAtom::Unavailable => return Err(ProtocolError::UnavailableEvidence),
            };
        }
        Ok(ObservationPacket::new(atoms))
    }
}
