use reqwest::header;

fn main() {
    let x = 3.14;
    let y = 1_f64 / x;

    // ruleid: reqwest-accept-invalid
    let client = reqwest::Client::builder()
            .build();

    // ruleid: reqwest-accept-invalid
    let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build();

}




