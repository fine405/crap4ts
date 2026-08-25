pub fn crap_score(complexity: u32, coverage: f64) -> f64 {
    let complexity = f64::from(complexity);
    let coverage = coverage.clamp(0.0, 1.0);
    complexity.powi(2) * (1.0 - coverage).powi(3) + complexity
}

#[cfg(test)]
mod tests {
    use super::crap_score;

    #[test]
    fn full_coverage_reduces_crap_to_complexity() {
        assert_eq!(crap_score(10, 1.0), 10.0);
    }

    #[test]
    fn no_coverage_applies_the_full_penalty() {
        assert_eq!(crap_score(10, 0.0), 110.0);
    }

    #[test]
    fn half_coverage_uses_a_cubic_discount() {
        assert_eq!(crap_score(4, 0.5), 6.0);
    }
}
