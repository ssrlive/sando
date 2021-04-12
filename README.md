# tokio-sando

This is a simple proxy server implementation based on [tokio][tokio].

The proxy is a server sandwiched (sando in Japanese) between the client and the client's destination. It will ask for the data from the destination on behalf of its client. In another word, the *proxy* essentially acts as a VPN. Please see [how it works](#how-it-works) to know more.

## How to Run

Open two terminals. One for the server, one for the client.

1. Add `127.0.0.1 proxy.tokio.sando` in your `/etc/hosts`
    - `proxy.tokio.sando` is the host name registered in the `domain.crt`
2. On server-side, run `./server`.
3. On client side, run `./client`.
    - The `./client` is based on `curl`. Make sure its version is higher than `7.68.0`. Otherwise, we cannot send HTTP requests in parallel.

### Advance Setting

You can set the pattern for the *destination*'s URL, in regex expression. For example, you can run the following commands:

On server side:

```sh
cargo run -- 127.0.0.1:7878 --pkcs12 domain.p12 --password "^G#=QVbVhh7Bt8t9L" --destination-pattern "([a-z]).(mozilla|rust-lang).org"
```

On client side:

```sh
# Work
curl -vp --proxy "https://proxy.tokio.sando:7878" --proxy-cacert domain.crt "https://www.rust-lang.org/"
# Work
curl -vp --proxy "https://proxy.tokio.sando:7878" --proxy-cacert domain.crt "https://www.mozilla.org/en-US/"
# Won't work. Doesn't match "([a-z]).(mozilla|rust-lang).org" pattern
curl -vp --proxy "https://proxy.tokio.sando:7878" --proxy-cacert domain.crt "https://en.wikipedia.org/"
```

### Generate a Certificate

This repo has a built-in certificate. If the certificate is expired or you wish to generate your own certificate, run the commands below:

1. [Generate a Self-Signed Certificate][gen-self-signed-cert]

   ```sh
   openssl req \
       -newkey rsa:2048 -nodes -keyout domain.key \
       -x509 -days 365 -out domain.crt
   ```

   - Replace the `proxy.tokio.sando` in `client.sh` with the `Common Name` registered in the `domain.crt`
   - If you change the file name of `domain.crt`, please update it in `client.sh` as well
2. [Pack the certificate and the private key into a PKCS12 file][pack-p12]

   ```sh
   openssl pkcs12 \
       -inkey domain.key \
       -in domain.crt \
       -export -out domain.p12
   ```

   - Replace the password `^G#=QVbVhh7Bt8t9L` in the `server.sh` by your new password
   - If you change the file name of `domain.p12`, please update it in `server.sh` as well

3. Now the `server.sh` and `client.sh` should work with your own certificate

Here is a [server][server-cert-sample]-[client][client-cert-sample] example.

## How It Works

```txt
  client        proxy       destination
    |             |             |
    * <--- 1 ---> *             |
    |             |             |
    * ---- 2 ---> * ---- 3 ---> *
    |             |             |
    * <--- 5 ---- * <--- 4 ---- *
    |             |             |
    * ----------> * ----------> * relay data: client -> destination
    |             |             |
    * <---------- * <---------- * relay data: destination -> client
    |             |             |
```

1. TCP and TLS handshake between *client* and *proxy* (HTTPS now)
2. *client* sends a [HTTP CONNECT][http-connect] request to *proxy*
3. *proxy* establishs a TCP channel to the *client*'s *destination*
4. Once the above TCP channel is built
5. notify *client* the [HTTP tunnel][http-tunnel] is established
6. Now *proxy* can relay data between *client* and *destination* via the tunnel

### Use case

One use case for the *proxy* is the privacy-preserving service. Once the *proxy* builds the tunnel, the *client* can talk to the *destination* **anonymously** if the data relay via the tunnel is in HTTPS. The *destination* knows the *client*'s request, but it has no idea of who the *client* is. The *proxy* knows both who the *client* and the *destination* are, but it has no idea of what they are talking about.

A demo here is the giphy search. Open two terminals. One for the server, one for the client.

1. Add `127.0.0.1 proxy.tokio.sando` in your `/etc/hosts` if `127.0.0.1 proxy.tokio.sando` is not there yet
2. On server-side, run `./server`, or the following command if the server is for giphy service only

    ```sh
    cargo run -- 127.0.0.1:7878 \
      --pkcs12 domain.p12 \                 # or your own PKCS12 file
      --password "^G#=QVbVhh7Bt8t9L" \      # or your new password if using your own PKCS12 file
      --destination-pattern "api.giphy.com" # Server only accept HTTP CONNECT to "api.giphy.com"
    ```

3. On client side, run `./giphy_search.sh` to search the gif anonymously.

Please ensure the HTTP CONNECT request, which asks to relay data between the *client* and the *destination*, is in HTTPS protocol. Otherwise, the *proxy* will know the contents transferred between the *client* and the *destination* if tunnel communication is in HTTP since HTTP transfers data in plain text.

## TODO

- Privacy-aware
  - Find a way to force client to use HTTPS to transfer data in the tunnel
    - It can be told by sniffing the relayed data at least
- More configurable settings
  - Add timeout settings
    - Close the connection if it's pending for a while. This should help to end the connection having unknown errors
  - HTTP version check
    - The proxy should be able to ask a minimal HTTP version
  - Max parallel connections
    - The proxy should be able to limit the number of the parallel connections, for resources control
- Better error/response handling
  - why: Currently, the connection is aborted if a `std::io::Error` is thrown. The proxy should send the response to client indicating the error encountered, instead of dropping the connection silently. Once the TLS between client and proxy is established, the proxy should be able to send message back to client
  - Send a `400 Bad Request` or else to client if the client request is incomplete or invalid
    - Use a specific error enum for request parsing should be helpful
  - Return an error to client if proxy cannot connect to the client's destination
  - Return an error to client if having trouble to relay daya
    - Should have a timeout mechanism for relaying data in the tunnel
    - Use a specific error enum for tunnel module should be helpful
  - Add more detailed error message in proxy server response. Now the response only contains the status code
- Test
  - Add performance benchmark. Need to work with parallel requests
  - Add more edge cases for request parsing tests
  - Add tests for different kinds of error
- Traffic control / statistics
  - Real time traffic statistics monitor and control
  - Collect the transferred bytes even when getting an error, as long as the proxy has relayed the data
    - Now the proxy shows statistics only when the connection is completed successfully. Should know how many data has been transferred even when having an error in tunnel relay or other failures
- Memory
  - Better buffer size control
    - Th buffer sizes for parsing HTTP request and relaying data are fixed now. Need a more sophisticated control when having a vast amount of HTTP requests in parallel on a server with limited memory
- Log system
  - Log messages in a file instead of just printing them on the screen
  - Set different level for logs. Some logs are debugging-only
- Build time / Code size
  - Evaluate if we need a `full` feature from tokio
- Implement proxy with [tokio-rustls][tokio-rustls] or [async-tls][async-tls] (in another project)
  - tokio-rustls
    - Could start from this [https server](https://github.com/ChunMinChang/https-server-tokio-rustls)
    - It has slightly better workaround for [tls "split" issue](https://github.com/tokio-rs/tls/issues/40)
  - async-tls
    - Could start from this [https server](https://github.com/ChunMinChang/https-server-async)
    - Work with [async-std][async-std] instead of [tokio][tokio]
    - async-std doesn't provide `split()` so it will need a new mechanism to relay data
      - A workaround is [using `async_std::io::copy`](https://github.com/async-rs/async-std/blob/35f768166436112db97224e823b4ee610c81d6d6/docs/src/patterns/small-patterns.md)
      - Howeve it doesn't work all the time [unless you know that those streams are `async_std::net::TcpStream`](https://users.rust-lang.org/t/async-std-splitting-reader-writer-lifetimes/36427/8). `Arc` might be an [option](https://github.com/async-rs/async-std/issues/563#issuecomment-557032088)
  - Take a look to [other similar implementations](https://github.com/topics/http-proxy?l=rust)

[tokio]: https://github.com/tokio-rs/tokio

[gen-self-signed-cert]: https://www.digitalocean.com/community/tutorials/openssl-essentials-working-with-ssl-certificates-private-keys-and-csrs#generate-a-self-signed-certificate
[pack-p12]: https://www.digitalocean.com/community/tutorials/openssl-essentials-working-with-ssl-certificates-private-keys-and-csrs#convert-pem-to-pkcs12

[server-cert-sample]: https://github.com/ChunMinChang/https-server-tokio-native-tls/blob/77f1b3b8b3739e20d9bd27a1dfa86128d8c16311/server.sh
[client-cert-sample]: https://github.com/ChunMinChang/https-server-tokio-native-tls/blob/77f1b3b8b3739e20d9bd27a1dfa86128d8c16311/client.sh

[http-connect]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/CONNECT
[http-tunnel]: https://en.wikipedia.org/wiki/HTTP_tunnel

[tokio-rustls]: https://github.com/tokio-rs/tls/tree/master/tokio-rustls
[async-tls]: https://github.com/async-rs/async-tls
[async-std]: https://github.com/async-rs/async-std