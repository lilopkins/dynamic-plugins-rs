use std::ffi::CStr;

use dynamic_plugin::{libc, plugin_impl};

plugin_impl! {
    example_plugin_host::ExamplePlugin,

    fn do_a_thing() {
        println!("A thing has been done!");
    }

    unsafe fn say_hello(name: *const libc::c_char) -> bool {
        let name = CStr::from_ptr(name);
        println!("Hello, {}!", name.to_string_lossy());
        true
    }

    fn trigger_function(a_func: extern "C" fn(u32, u32)) {
        a_func(5, 3);
    }
}
