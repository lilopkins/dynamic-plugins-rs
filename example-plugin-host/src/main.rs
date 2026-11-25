use std::ffi::CString;

use dynamic_plugin::{PluginInterface, Result};
use example_plugin_host::ExamplePlugin;

extern "C" fn a_func(a: u32, b: u32) {
    println!("a = {a}, b = {b}");
    assert_eq!(a, 5);
    assert_eq!(b, 3);
}

fn load_generic_plugin<T: PluginInterface>() -> Result<T> {
    T::load_plugin_and_check("target/debug/libexample_plugin.so")
}

fn main() -> Result<()> {
    let plugin = load_generic_plugin::<ExamplePlugin>()?;

    plugin.do_a_thing()?;
    let s = CString::new("Jens").unwrap();
    plugin.say_hello(s.as_ptr())?;
    plugin.trigger_function(a_func)?;

    Ok(())
}
