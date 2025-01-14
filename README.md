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

### Running without nix

There _might_ be some way to run this without nix, you need to:

1. Build a custom version of openvpn using using these patches:
   <https://raw.githubusercontent.com/samm-git/aws-vpn-client/master/openvpn-v2.5.1-aws.patch>.
2. Point the vpn client to your custom version of openvpn using `export OPENVPN_FILE=???`.
