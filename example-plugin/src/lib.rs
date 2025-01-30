use std::ffi::CStr;

use dynamic_plugin::{libc, plugin_impl};

plugin_impl! {
    example_plugin_host::ExamplePlugin,

    fn do_a_thing() {
        println!("A thing has been done!");
    }

    fn say_hello(name: *const libc::c_char) -> bool {
        unsafe {
            let name = CStr::from_ptr(name);
            println!("Hello, {}!", name.to_string_lossy());
        }
        true
    }
}
