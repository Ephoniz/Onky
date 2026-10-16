use super::short;

#[test]
fn strip_label_drops_the_product_prefix() {
    assert_eq!(short("Audio Program Virtual Input 1"), "Virtual Input 1");
    assert_eq!(short("Mic"), "Mic");
}
