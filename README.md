# Dynamically Loaded Plugins for Rust

## Goal

In many pieces of software, it can be beneficial to allow other developers to add functionality that you have not considered. In order to do this, plugin-like systems are often used.

Rust is a powerful language with a strong emphasis on safety, however this can make working with plugins quite challenging as you can quickly lose a lot of the compile-time checking that Rust offers. The goal of this library is to reintroduce the safety that Rust is built upon when writing plugin libraries (DLLs, DyLibs or SOs) that are loaded at runtime.

## Architecture

```text
             ┌───────────┐
    Safe     │           │  Compile-time
    Interface│ Reusable  │  Checking
     ┌──────►│  Plugin   │◄─────┬────────┐
     │       │ Interface │      │        │
     │       │           │      │        │
┌────┴───┐   └───────────┘ ┌────┴─────┐  │
│        │                 │          │  │
│ Plugin │◄───────────────►│ Plugin A │  │
│  Host  │  Runtime calls  │          │  │
│        │◄───────┐        └──────────┘  │
└────────┘        │                      │
                  │        ┌──────────┐  │
                  │        │          │  │
                  └───────►│ Plugin B ├──┘
                           │          │
                           └──────────┘
```

### Plugin Host

The plugin host is the part of the system that will find and load the plugins at runtime and will call upon their functionality. This is usually your main software package.

### Plugin Client

The plugin client(s) are the plugins that are written by yourself/other developers. These must be written to match the plugin interface provided by the host and allow a
safe way to call upon other code.

## Writing a Plugin System

To write a plugin system, you will first need to decide upon your interface. For this example, we'll demonstrate with this interface:

```text
┌───────────────────────────────┐
│ExamplePlugin                  │
├───────────────────────────────┤
│do_a_thing()                   │
│say_hello(name: string) -> bool│
└───────────────────────────────┘
```

The `do_a_thing` function here will just be triggered to do whatever the plugin author decides. The `say_hello` function should display a message to the named person, then return a boolean as to whether that was successful. Admittedly, this isn't exactly a complex interface!

### Writing a Plugin Host

In your project, add the `dynamic-plugin` library:

```sh
cargo add dynamic-plugin --features host
```

Now in your `main.rs` file, you can define your interface:

```ignore
use dynamic_plugin::{libc::c_char, plugin_interface};

plugin_interface! {
    extern trait ExamplePlugin {
        /// Ask the plugin to do a thing
        fn do_a_thing();
        /// Say hello to a person
        fn say_hello(to: *const c_char) -> bool;
    }
}
```

Note that we can't just send strings around! As this depends upon FFI, we need to use C-compatible data. Rust will warn you if you do not do this!

That is almost it! We can now write some code to actually use these plugins:

```ignore
fn main() -> dynamic_plugin::Result<()> {
    let plugins = ExamplePlugin::find_plugins("./plugins")?;
    for plugin in plugins {
         plugin.do_a_thing()?;
         let s = std::ffi::CString::new("Jens").unwrap();
         plugin.say_hello(s.as_ptr())?;
    }
    Ok(())
}
```

### Writing a Plugin Client

You can now write plugins for your interface! Create a new library project:

```sh
cargo new --lib example-plugin
```

In `Cargo.toml`, specify that this should build as a C-compatible library:

```toml
[lib]
crate-type = [ "cdylib" ]

[dependencies]
dynamic-plugin = { version = "x.x.x", features = [ "client" ] }
```

You can now define your plugin implementation:

```ignore
use std::ffi::CStr;
use dynamic_plugin::{libc::c_char, plugin_interface, plugin_impl};

plugin_interface! {
    extern struct ExamplePlugin {
        /// Ask the plugin to do a thing
        fn do_a_thing();
        /// Say hello to a person
        fn say_hello(to: *const c_char) -> bool;
    }
}

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
```

The plugin is now ready to build and distribute.

### Taking this further...

You can also avoid reusing the plugin definition by putting it in it's own library. An implementation that does this is available in the `example-plugin` and `example-plugin-host` folders of the source repository.

## `attempt to compute '0_usize - 1_usize', which would overflow`

If you come across this compile-time error, this indicates that the implementation you are writing does not match the expected implementation for the plugin definition. Please check that you:

- Are using the correct definition.
- Have all the functions you need to meet the definition.
- That all the functions are named correctly (identically to the definition).
- That all the function arguments are the same order and types as the definition.
- That all the function return types are the same as the definition.
