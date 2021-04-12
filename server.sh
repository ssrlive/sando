pkcs12_file="domain.p12"
pkcs12_file_password="^G#=QVbVhh7Bt8t9L"
cargo run -- 127.0.0.1:7878 --pkcs12 $pkcs12_file --password $pkcs12_file_password #--destination-pattern "([a-z]).(mozilla|rust-lang).org"