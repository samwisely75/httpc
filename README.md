# webcat

[![GitHub build](https://github.com/blueeaglesam/webcat/actions/workflows/rust.yml/badge.svg)](https://github.com/blueeaglesam/webcat/actions/workflows/rust.yml)

A light-weight profile-based HTTP client allows you to talk to web servers with a minimal effort.

## Usage

### Basics

The simplest usage is to run the following command in your terminal: 

```sh
webcat GET /
```

You can `PUT` or `POST` with simple command like this:

```sh
webcat PUT _cluster/settings '{
    "persistent": {
        "cluster.routing.allocation.enabled": "primaries"
    }
}'
```

### Use Standard Input

You can also pass a request body via standard input: 

```sh
echo '{
    "query": {
        "range": {
        "@timestamp": {
            "gte": "now-1d/d",
            "lt": "now/d"
        }
    }
}' \
| webcat GET my-index/_search \
| jq -r '.hits.hits[] | [ ._id, ._source.name ] | @tsv' \
| sed -e "s/\t/ /g"
```

### Switch Connection via Profile

The commands above are using `default` profile in your `~/.webcat`, a configuration file that contains the base url (e.g. `https://my-remote-server:9200`), user, password, and other parameters. You can switch the context by specifying another profile name in `--profile` (or `-p`). 

```sh
webcat -p my-dev-cluster GET /_cluster/settings
webcat -p cust-qa-cluster GET /_cluster/settings
```

### Override/Aurgment Configurations

 You can run the command without configuring `~/.webcat` by providing a URL starts with `http://` or `https://` directly into the `URL` like `curl`. You can also use other command line parameters such as `--user` (or `-u`) and `--password` (or `-w`) to augment/override the configuration.

```sh
webcat GET https://my-local-server:9200/_cluster/health \
    --user elastic \
    --password changeme \
    --ca-cert /path/to/ca.pem
```

For all available command line options, run `webcat -h` or `webcat --help`.

## Features

- Curl-like command line options allows to talk to web servers
- Enable minimizing parameters 
- Support multiple connection profiles and select by `-p` parameter
- Support all HTTP methods (GET, POST, PUT, DELETE, etc.)
- Support auto redirection
- Support standard input for request body
- Support multiple compression (gzip, deflate, zstd)
- Support multiple headers
- Support custom CA certtificate for SSL/TLS
- Support request through a HTTP proxy that doesn't require authentication
- Enable skipping SSL/TLS server certificate validation
- Provide verbose mode writes the details of request and response to the error output

## Installation

1. Download the binary in releases according to your platform.
1. Expand .tar.gz and copy `webcat` to where `$PATH` is thru (e.g. `/usr/local/bin`)
1. Run `webcat --help` to test it.

## Configuration

The configuration can be done through `~/.webcat` file, which is in a good-old INI format that contains more than one profiles, consisting of more than one key-value pairs. You can switch the profile by specifying `--profile` (or `-p`) command line option. If you don't specify the profile, `default` will be used.

```ini
[default]
host = https://elastic-prod.es.us-central1.gcp.cloud.es.io
user = elastic
password = changeme
insecure = false
ca_cert = /path/to/ca.pem
@content-type = application/json
@user-agent = webcat/0.1
@accept = */*
@accept-encoding = gzip, deflate
@accept-langugage = en-US,en;q=0.9

[local]
host = http://localhost:9200
user = elastic
password = changeme
```

The entities start with `@` will be treated as HTTP headers. You can specify multiple headers by adding more keys with the same prefix. 

## Enhancement Plan

- [x] Remove `--stdin` parameter
- [x] Support multiple headers in command line options
- [ ] Introduce blank profile
- [ ] Support proxy
- [ ] Support client certificate authentication
- [ ] Support binary data send
- [ ] Support multi-form post
- [ ] REPL capability with cookie support
- [ ] Beautify JSON output

## Motivation

I am a consultant and I talk to Elasticsearch every day. Kibana Dev Tools is the primary option, however on the consulting field it's not always available. 

`curl` works great in that situation, however providing same parameters such as `-u elastic:password` and `-H "content-type: application/json"` every time I query the node is painful. The scheme defintion, host name, and port number in the URL are redundant too. 

In Kibana Dev Tools you can say:

`GET /_cat/indices?v` 

which in `curl` becomes like:

```sh
curl -XGET \
     -u "elastic:password" \
     -H "content-type: application/json" \
     https://prod-cluster.es.us-central1.gcp.cloud.es.io/_cat/indices?v
```

I wanted to bring the simplicity of Kibana Dev Tools to `curl` and `httpie` users.

I know Python does the job and I've been there. The problem is that the source code will soon become lengthy and difficult to maintain in a single file, so you will need to carry around multiple files to make it work. You also need `requests` library to be installed, which is hassle for the vintage OSs do not have `pip` at the start. This isn't cool.

## Contribution

Contributions and bug reports are welcome. Please feel free to open an issue or send me a pull request.

