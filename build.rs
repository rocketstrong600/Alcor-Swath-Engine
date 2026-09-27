use proc_macro2::Literal;
use quote::quote;
use roxmltree::Document;
use std::{env, fs, path::Path, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=vk.xml");
    println!("cargo:rerun-if-changed=build.rs");

    let xml_data = fs::read_to_string("vk.xml").expect("Failed to read vk.xml");
    let doc = Document::parse(&xml_data).expect("Failed to parse vk.xml");

    let mut match_arms = quote! {};

    for node in doc.descendants().filter(|n| n.has_tag_name("extension")) {
        if let Some(ext_name) = node.attribute("name") {
            if let Some(ext_promotedto) = node.attribute("promotedto") {
                if let Some(ext_promotedto_rest) = ext_promotedto.strip_prefix("VK_VERSION_") {
                    let (major, minor) = ext_promotedto_rest
                        .split_once('_')
                        .and_then(|(maj_str, min_str)| {
                            let maj: u32 = maj_str.parse().ok()?;
                            let min: u32 = min_str.parse().ok()?;
                            Some((maj, min))
                        })
                        .expect("Failed to parse vk_version");

                    let ext_name_bytes = Literal::byte_string(ext_name.as_bytes());

                    let match_arm = quote! {
                        #ext_name_bytes => Some(ash::vk::make_api_version(0, #major, #minor, 0)),
                    };

                    match_arms.extend(match_arm);
                }
            }
        }
    }

    let generated_code = quote! {
        pub mod extensions_info {
            use std::ffi::CStr;
            pub fn get_ext_promoted_version(ext_name: &CStr) -> Option<u32> {
                match ext_name.to_bytes() {
                    #match_arms
                    _ => None,
                }
            }
        }
    };

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_vulkan.rs");
    fs::write(&dest_path, generated_code.to_string()).expect("Failed to write generated code");
    println!("Generated rust code to: {}", dest_path.display());

    if let Err(_) = Command::new("rustfmt").arg(&dest_path).status() {
        println!("cargo:warning=rustfmt failed to format generated code")
    }
}
