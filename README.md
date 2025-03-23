# wiq (web interface query)

![GitHub build](https://github.com/elasticsatch/wiq/actions/workflows/rust.yml/badge.svg)

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

You can also pass a request body data through the standard input with specifying `--stdin` (or `-i`).

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
| wiq -i GET my-index/_search \
| jq -r '.hits.hits[] | ._source | [ .name, .age ] | @tsv' \
| column -t
```

The commands above are using `default` profile in your `~/.wiq`, a configuration file that contains the base url (e.g. `https://my-remote-server:9200`), user, password, and other parameters. You can switch the connection by specifying `--profile` (or `-p`) parameter. 

```sh
wiq -p my-dev-cluster GET /_cluster/settings
wiq -p cust-qa-cluster GET /_cluster/settings
```

If you don't have a `default` profile, it'll prompt you to create one at the first attempt. 

You can run the command without configuring `~/.wiq` by providing a URL starts with `http://` or `https://` directly into the `URL` parameter, like `curl`. You can also use other command line parameters such as `--user` (or `-u`) and `--password` (or `-w`) to augment/override the configuration. 

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
@accept = application/json
@user-agent = wiq/0.1
```

Entity starts with `@` will be treated as HTTP header. You can specify multiple headers by adding more keys with the same prefix. 

## Background / Motivation

I am a consultant at Elasticsearch and I talk to Elasticsearch every day. Kibana Dev Tools is the primary option, however on the field of consulting it's not always available. 

For example, when I got stuck while building a fresh self-hosted ES cluster, I open up a terminal window and ssh to one of the node, check the systemd status, tail the log, and check elasticsearch.yml. Often time I need to run some diagnostic queries agaist the cluster to check its internal state. If the ES node is up and running it's on `https://localhost:9200`, otherwise I need to talk to other nodes. 

`curl` works great for that needs, but one thing I don't like is to provide all static parameters such as `-u elastic:password`, `-H "content-type: application/json"`, or `--insecure` every time I query the node. The scheme defintion, host name, and port number in the URL are redundant too. 

In Kibana Dev Tools you simply say:

`GET /_cat/indices?v` 

but in `curl` it'll be:

`curl -XGET -u elastic:password -H "content-type: application/json" https://elastic-prod.es.us-central1.gcp.cloud.es.io/_cat/indices?v` 

This is painful even for a terminal guy like me. Why can't I bring the Dev Tools experience to the terminal?

Also, I occasionally need to talk to two or three different clusters at the same time. I often launch multiple terminals to talk to a cluster in a window, and I easily get lost in which window is talking to which cluster. Here, I wanted to have a profile system that allows me to switch the counterpart easily in a single terminal, like `aws-cli` does.

Yes, Bash or Python does the job. I've been there and done that. The problem is that it will soon become lengthy and difficult to maintain. I also needed to make it work with Python 2.6 or 2.7 for someone using vintage OSs, and I had hard time to maintain the compatibility across different/old version of Pythons. 

I have been playing with Rust and thought it would be a goood opportunity to implement it with it. The advantage of Rust is that it's fast, as fast as native C/C++ tools including curl, which is essential for this kind of stuff. And you won't suffer from the compatibility issue like Python's case.

## Contribution

Contributions and bug reports are welcome. Please feel free to open an issue or pull request.
