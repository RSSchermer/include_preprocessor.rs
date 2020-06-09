use proc_macro_hack::proc_macro_hack;

#[proc_macro_hack(fake_call_site)]
pub use include_preprocessor_macro_impl::include_str_ipp;
