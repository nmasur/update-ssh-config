# Update SSH Config

Modifies ~/.ssh/config file; finds and replaces the hostname for the given host.

## Usage

```
$ update-ssh-config --host <host> --hostname <new hostname>
```

Example:

```
$ update-ssh-config --host prodserver --hostname 54.26.33.20
```

Bash function using [drips](https://github.com/nmasur/drips) to get latest IP, update config, and then SSH:

```
connect() {
    search=$1
    results=$(drips --raw | grep $search)
    ip=$(echo $results | sed -n 1p | awk -F' - ' '{print $2}')
    update-ssh-config --host $search --hostname $ip
    ssh $search
}

$ connect myhost
```

## Install

See [releases](https://github.com/nmasur/update-ssh-config/releases) page for binaries.

On MacOS, you can also install from Homebrew:

```
brew tap nmasur/repo
brew install nmasur/repo/update-ssh-config
```

Alternatively, build from source using [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):

```
git clone git://github.com/nmasur/update-ssh-config
cd update-ssh-config
cargo build --release
```
