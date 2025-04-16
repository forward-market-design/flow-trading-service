use crate::Submission;
use std::fmt::Display;
use std::hash::Hash;
use std::io::Write;

/// Convert a set of flow trading submissions to a quadratic program and export
/// this program to `.mps` format.
pub fn export_mps<
    T,
    BidderId: Display + Eq + Hash + Clone + Ord,
    PortfolioId: Display + Eq + Hash + Clone + Ord,
    ProductId: Display + Eq + Hash + Clone + Ord,
>(
    auction: &T,
    buffer: impl Write,
) -> Result<(), std::io::Error>
where
    for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<PortfolioId, ProductId>)>,
{
    unimplemented!(".mps export todo");

    Ok(())
}

/// Convert a set of flow trading submissions to a quadratic program and export
/// this program to `.lp` format.
pub fn export_lp<
    T,
    BidderId: Display + Eq + Hash + Clone + Ord,
    PortfolioId: Display + Eq + Hash + Clone + Ord,
    ProductId: Display + Eq + Hash + Clone + Ord,
>(
    auction: &T,
    buffer: impl Write,
) -> Result<(), std::io::Error>
where
    for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<PortfolioId, ProductId>)>,
{
    unimplemented!(".lp export todo");

    Ok(())
}
