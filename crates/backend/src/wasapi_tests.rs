use super::WasapiBackend;

#[test]
fn connect_leaves_the_ui_thread_in_a_single_threaded_apartment() {
    WasapiBackend::connect().expect("COM STA init");
    let multithreaded = wasapi::initialize_mta();
    assert!(
        multithreaded.is_err(),
        "the UI thread was switched to the MTA, so the window cannot open"
    );
}
