**Disclaimer**: This is an unofficial implementation of an AWS VPN Client for
Linux. It does not have any relation with AWS in any way.

# Harry's AWS VPN client

Reimplementation of <https://github.com/JonathanxD/openaws-vpn-client> which
itself was a reimplementation of <https://github.com/samm-git/aws-vpn-client>.

Not suitable for using.

## Run

1. Install nix
2. Download an .ovpn file from a AWS VPN self-service portal.
3. Run
    ```
    nix run . -- /home/harrysarson/cvp*n
    ```

There might be some way to run this without nix, the steps above are what I do.
