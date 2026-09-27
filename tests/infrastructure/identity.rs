use buildpilot::infrastructure::locations::DONATE_URL;

// UI-014: the donation address, letter for letter, so a slip in it fails here rather than
// sending a supporter to a page that is not the author's; and only ever over https.
#[test]
fn the_donation_address_is_the_authors() {
    assert_eq!(
        DONATE_URL,
        "https://www.paypal.com/ncp/payment/XC9S6VZ96K9Q4"
    );
    assert!(DONATE_URL.starts_with("https://"));
}
