use std::ffi::CString;

use dynamic_plugin::Result;
use example_plugin_host::ExamplePlugin;

extern "C" fn a_func(a: u32, b: u32) {
    println!("a = {a}, b = {b}");
    assert_eq!(a, 5);
    assert_eq!(b, 3);
}

fn main() -> Result<()> {
    let plugin = ExamplePlugin::load_plugin_and_check("target/debug/libexample_plugin.so")?;

    plugin.do_a_thing()?;
    let s = CString::new("Jens").unwrap();
    plugin.say_hello(s.as_ptr())?;
    plugin.trigger_function(a_func)?;

    Ok(())
}
