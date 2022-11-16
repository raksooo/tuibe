# Tuibe

Tuibe is a TUI for browsing RSS feeds such as a youtube channel feed. It's useful for keeping track
of YouTube subscriptions.

## Installation
```sh
cargo install --git https://github.com/raksooo/tuibe
```

## Usage
```
$ tuibe --help
Available options:
  -h, --help                Show this help message.
  --import-youtube <path>   Import subscriptions csv from YouTube takeout
  --player <player>         Override player in config
```

## Todo
- Design improvements
- Copy url for currently selected video or feed
- Combine `handle_event` and `registered_events`
- Add support for RSS in addition to ATOM
- Add support for other RSS feeds than YouTube
- Add backend for YouTube API
- Add flag to run with different backend
- Correctly handle emojis in feed name and video name

