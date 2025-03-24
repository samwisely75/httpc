# wiq

[![GitHub build](https://github.com/elasticsatch/wiq/actions/workflows/rust.yml/badge.svg)](https://github.com/elasticsatch/wiq/actions/workflows/rust.yml)

A light-weight profile-based HTTP client allows you to talk to web servers with a minimal effort. 

## Usage

The simplest usage is to run the following command in your terminal: 

```sh
wiq GET /
```

You can `PUT` or `POST` with simple command like this:

```sh
wiq PUT _cluster/settings '{
    "persistent": {
        "cluster.routing.allocation.enabled": "primaries"
    }
}'
```

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
| wiq GET my-index/_search \
| jq -r '.hits.hits[] | [ ._id, ._source.name ] | @tsv' \
| column -t
```

The commands above are using `default` profile in your `~/.wiq`, a configuration file that contains the base url (e.g. `https://my-remote-server:9200`), user, password, and other parameters. You can switch the context by specifying another profile name in `--profile` (or `-p`). 

```sh
wiq -p my-dev-cluster GET /_cluster/settings
wiq -p cust-qa-cluster GET /_cluster/settings
```

If you don't have a `default` profile, it'll ask you to create one.  You can run the command without configuring `~/.wiq` by providing a URL starts with `http://` or `https://` directly into the `URL` like `curl`. You can also use other command line parameters such as `--user` (or `-u`) and `--password` (or `-w`) to augment/override the configuration. 

```sh
wiq GET https://my-local-server:9200/_cluster/health \
    --user elastic \
    --password changeme \
    --ca-cert /path/to/ca.pem
```

For all available command line options, run `wiq -h` or `wiq --help`.

## Installation

1. Download the binary in releases according to your platform.
1. Expand .tar.gz and copy `wiq` to where `$PATH` is thru (e.g. `/usr/local/bin`)
1. Run `wiq -h` to test it.

## Configuration

The configuration can be done through `~/.wiq` file, which is in a good-old INI format that contains more than one profiles, consisting of more than one key-value pairs. You can switch the profile by specifying `--profile` (or `-p`) command line option. If you don't specify the profile, `default` will be used.

```ini
[default]
host = https://elastic-prod.es.us-central1.gcp.cloud.es.io
user = elastic
password = changeme
insecure = false
ca_cert = /path/to/ca.pem
@content-type = application/json
@user-agent = wiq/0.1
@accept = */*
@accept-encoding = gzip, deflate
@accept-langugage = en-US,en;q=0.9
```

Entities start with `@` will be treated as HTTP headers. You can specify multiple headers by adding more keys with the same prefix. 

## Motivation

I am a consultant at Elasticsearch and I talk to Elasticsearch every day. Kibana Dev Tools is the primary option, however on the field of consulting it's not always available. 

`curl` works great for that needs, but one thing I don't like is to provide all static parameters such as `-u elastic:password` and `-H "content-type: application/json"` every time I query the node. The scheme defintion, host name, and port number in the URL are redundant too. 

In Kibana Dev Tools you simply say:

`GET /_cat/indices?v` 

but in `curl` it'll be:

`curl -XGET -u elastic:password -H "content-type: application/json" https://elastic-prod.es.us-central1.gcp.cloud.es.io/_cat/indices?v` 

This is painful even for a command-line maniac like me. 

I know Python does the job and I've been there. The problem is that it will soon become lengthy and difficult to maintain in a single file, so you will need to carry around multiple files to make it run. This isn't cool.

## Enhancement Plan

- [x] Remove `--stdin` parameter
- [ ] Support proxy
- [ ] Support multiple headers in command line options as curl does
- [ ] Client certificate authentication with `--client-cert`
- [ ] REPL capability

## Contribution

Contributions and bug reports are welcome. Please feel free to open an issue or send me a pull request.

