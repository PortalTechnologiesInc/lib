# Portal Wallet API

Supported types:

- NWC
- Breez

## Prerequisites

The `breez-sdk-spark` dependency includes `tonic-build` as a build dependency, which uses `prost-build` to compile Protocol Buffer files.
You need to install the protobuf development files. 

On Fedora, run:

```
sudo dnf install protobuf-devel
```

This will install the Google protobuf `.proto` files that `tonic-build` needs.
