# http
Light-weight profile-based HTTP client enables you to write a request in natural order.

![build](https://github.com/elasticsatch/http/actions/workflows/rust.yml/badge.svg)

## Setup 

1. Download the binary in releases
1. Expand .tar.gz and copy `http` to where `$PATH` is thru (e.g. `/usr/local/bin`)
1. Run `http -h` to test it

## Usage

The simplest use is to run the following command in your terminal: 

```sh
http GET /
```

It issues the given HTTP request based on the profile configured in your `~/.http` and print the response. If the `~/.http` is not found and if the URL parameter doesn't start with `http`, it'll prompt to create one.



