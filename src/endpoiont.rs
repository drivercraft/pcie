use crate::{root::RootComplex, Chip};

pub struct PciEndpoint<'a, C: Chip> {
    root: &'a RootComplex<C>,
}
