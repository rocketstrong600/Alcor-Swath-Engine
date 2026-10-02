pub mod app;
pub mod renderer;
pub mod utils;

pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated_vk_info.rs"));
}
