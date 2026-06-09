use libqalculate_rust::ffi::Calculator;

fn assert_sync<T: Sync>() {}

fn main() {
    assert_sync::<Calculator>();
    let calc = Calculator::new();
    let calc_ref = &calc;
    std::thread::spawn(move || {
        let _ = calc_ref;
    });
}
