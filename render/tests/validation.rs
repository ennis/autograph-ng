//! validation tests
use autograph_render_test::with_test_fixture;

#[test]
fn test_simple() {
    with_test_fixture("test_simple", Some(60), |r, a, run| {})
}
