crt_file="domain.crt"
domain="proxy.tokio.sando"

# GET request
curl -vp --cacert $crt_file "https://$domain:7878"

# CONNECT request to a nonexistent URL
curl -vp --proxy "https://$domain:7878" --proxy-cacert $crt_file "https://404.not.found"

# CONNECT request
curl -vp --proxy "https://$domain:7878" --proxy-cacert $crt_file "https://www.mozilla.org/en-US/"

# Parallel CONNECT requests (only works for curl >= 7.68.0)
curl --proxy "https://$domain:7878"  --proxy-cacert $crt_file \
  --parallel --parallel-immediate --parallel-max 10 \
  --url "https://www.rust-lang.org/" \
  --url "https://foundation.rust-lang.org/" \
  --url "https://github.com/rust-lang" \
  --url "https://tokio.rs/" \
  --url "https://github.com/tokio-rs/tokio"