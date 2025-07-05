use tectonic;
use tectonic::{config, driver, status};
use cxx::UniquePtr;
use std::fmt::Arguments;
use crate::tectonic_ffi::CppStatusBackend;

// Add `pub` to make this function part of the crate's public API
pub fn run_latex_from_string(latex: &str, status_backend: UniquePtr<CppStatusBackend>) -> Vec<u8> {
    let mut status = CxxStatusBackend {
        cpp_side: status_backend,
    };

    let auto_create_config_file = false;
    let config = config::PersistentConfig::open(auto_create_config_file).expect("Cannot open persistent config");

    let only_cached = false;
    let bundle = config.default_bundle(only_cached, &mut status).expect("Cannot open default bundle");

    let format_cache_path = config.format_cache_path().expect("Cannot get format cache path");

    let mut files = {
        // Looking forward to non-lexical lifetimes!
        let mut sb = driver::ProcessingSessionBuilder::default();
        sb.bundle(bundle)
            .primary_input_buffer(latex.as_bytes())
            .tex_input_name("texput.tex")
            .format_name("latex")
            .format_cache_path(format_cache_path)
            .keep_logs(false)
            .keep_intermediates(false)
            .print_stdout(false)
            .output_format(driver::OutputFormat::Pdf)
            .do_not_write_output_files();

        let mut sess =
            sb.create(&mut status).expect("Failed to initialize LaTeX processing session");
        sess.run(&mut status).expect("the LaTeX engine failed");
        sess.into_file_data()
    };

    match files.remove("texput.pdf") {
        Some(file) => file.data,
        None => Vec::new(),
    }
}

#[cxx::bridge]
mod tectonic_ffi {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum MessageKind {
        Note,
        Warning,
        Error,
    }
    extern "Rust" {
        fn run_latex_from_string(latex: &str, status_backend: UniquePtr<CppStatusBackend>) -> Vec<u8>;
    }

    unsafe extern "C++" {
        include!("tectonic-cxx-interface.h");

        type CppStatusBackend;

        fn report(
            self: Pin<&mut CppStatusBackend>,
            kind: MessageKind,
            message: &str,
        );
        fn dump_error_logs(self: Pin<&mut CppStatusBackend>, output: &[u8]);
    }
}

struct CxxStatusBackend {
    cpp_side: UniquePtr<tectonic_ffi::CppStatusBackend>,
}

impl status::StatusBackend for CxxStatusBackend {
    fn report(&mut self, kind: status::MessageKind, args: Arguments, err: Option<&anyhow::Error>) {
        // Convert tectonic MessageKind to our FFI MessageKind
        let ffi_kind = match kind {
            status::MessageKind::Note => tectonic_ffi::MessageKind::Note,
            status::MessageKind::Warning => tectonic_ffi::MessageKind::Warning,
            status::MessageKind::Error => tectonic_ffi::MessageKind::Error,
        };
        let message = if let Some(error) = err {
            format!("{}: {}", args, error)
        } else {
            args.to_string()
        };
        self.cpp_side.pin_mut().report(ffi_kind, &message);
    }

    fn dump_error_logs(&mut self, output: &[u8]) {
        self.cpp_side.pin_mut().dump_error_logs(output);
    }
}