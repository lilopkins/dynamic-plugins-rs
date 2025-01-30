use std::ffi::CString;

use dynamic_plugin::Result;
use example_plugin_host::ExamplePlugin;

fn main() -> Result<()> {
    let plugin = ExamplePlugin::load_plugin_and_check("../example-plugin/target/debug/libexample_plugin.so")?;

    plugin.do_a_thing()?;
    let s = CString::new("Jens").unwrap();
    plugin.say_hello(s.as_ptr())?;

    Ok(())
}
