use libqalculate_rust::ffi::Calculator;

fn assert_send<T: Send>() {}

fn main() {
    assert_send::<Calculator>();
    let calc = Calculator::new();
    std::thread::spawn(move || {
        let _ = calc;
    });
}
