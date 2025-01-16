use std::ffi::CStr;

use dynamic_plugin::{libc::c_char, plugin_impl};
use example_plugin_host::ExamplePlugin;

plugin_impl! {
    ExamplePlugin,

    fn do_a_thing() {
        println!("A thing has been done!");
    }

    fn say_hello(name: *const c_char) -> bool {
        unsafe {
            let name = CStr::from_ptr(name);
            println!("Hello, {}!", name.to_string_lossy());
        }
        true
    }
}
