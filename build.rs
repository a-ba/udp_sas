extern crate gcc;

fn main() {
    gcc::Build::new()
        .file("src/udp_sas.c")
        .compile("librust_udp_sas.a");
}
