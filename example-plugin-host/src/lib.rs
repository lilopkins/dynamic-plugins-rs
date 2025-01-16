use dynamic_plugin::{libc::c_char, plugin_interface};

plugin_interface! {
    extern trait ExamplePlugin {
        /// Ask the plugin to do a thing
        fn do_a_thing();
        /// Say hello to a person
        fn say_hello(to: *const c_char) -> bool;
    }
}
