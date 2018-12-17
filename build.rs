extern crate cc;

fn main() {
    cc::Build::new()
        .file("src/udp_sas.c")
        .compile("librust_udp_sas.a");
}
