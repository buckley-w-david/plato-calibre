# plato-calibre

This is a [Plato hook](https://github.com/baskerville/plato/blob/master/doc/HOOKS.md) to download/sync books from calibre.

It will automatically fetch books from a specified category in a [calibre content server](https://manual.calibre-ebook.com/server.html) instance.

Currently it depends on features only in [my fork of Plato](https://github.com/buckley-w-david/plato/tree/update-document-event), but I hope to get my changes upstreamed.

## Building

### Prep
0. Clone this repo.
1. Download the `gcc-linaro-4.9.4-2017.01` release for whatever your environment is [here](https://releases.linaro.org/components/toolchain/binaries/4.9-2017.01/arm-linux-gnueabihf/). Decompress the archive and have the `bin` directory in there somewhere on your path.

### Compile

```bash
$ cargo build --release --target=arm-unknown-linux-gnueabihf
```

## "Install"

```bash
# Create plato-calibre directory
$ mkdir -p "$KOBO_MOUNT_DIR/.adds/plato/bin/plato-calibre/"
# Copy the binary over
$ cp target/arm-unknown-linux-gnueabihf/release/plato-calibre "$KOBO_MOUNT_DIR/.adds/plato/bin/plato-calibre/"
# Copy over the settings (After putting in your info)
$ cp Settings-sample.toml "$KOBO_MOUNT_DIR/.adds/plato/bin/plato-calibre/Settings.toml"
```

Alternativly, you can try the `deploy.sh` script in the repo.

Once that's done you'll need to add the hook to your Plato settings file `$KOBO_MOUNT_DIR/.adds/plato/Settings.toml`.
```toml
[[libraries.hooks]]
path = "Calibre"
program = "bin/plato-calibre/plato-calibre"
sort-method = "added"
first-column = "title-and-author"
second-column = "progress"
```

## Settings

The `Settings-sample.toml` file contains the settings you will need to set before the hook can work.

### Finding "category" and "item"

There are two fields in the settings, `category` and `item`, that indicate what set of books should be synced. These are just integers, and as far as I know there isn't a really nice way to find them, I got mine from making API calls to my calibre-server instance.

```python
>>> import urllib.request, base64
>>> CALIBRE_SERVER_URL = "https://..."
>>> auth = base64.b64encode("username:password".encode()).decode()
>>> # Have to set a User-Agent or you just get a 403, I just stole mine from my browser
>>> headers = {"Authorization": f"Basic {auth}", "User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:94.0) Gecko/20100101 Firefox/94.0"}
>>> # Fetch the list of categories
>>> urllib.request.urlopen(urllib.request.Request(f"{CALIBRE_SERVER_URL}/ajax/categories/", headers=headers)).read().decode()
'[{"name": "Newest", "url": "/ajax/category/6e6577657374/calibre-library", "icon": "/icon/forward.png", "is_category": false}, {"name": "All books", "url": "/ajax/category/616c6c626f6f6b73/calibre-library", "icon": "/icon/book.png", "is_category": false}, {"url": "/ajax/category/617574686f7273/calibre-library", "name": "Authors", "icon": "/icon/user_profile.png", "is_category": true}, {"url": "/ajax/category/2367656e7265/calibre-library", "name": "Genre", "icon": "/icon/column.png", "is_category": true}, {"url": "/ajax/category/6c616e677561676573/calibre-library", "name": "Languages", "icon": "/icon/languages.png", "is_category": true}, {"url": "/ajax/category/7075626c6973686572/calibre-library", "name": "Publisher", "icon": "/icon/publisher.png", "is_category": true}, {"url": "/ajax/category/726174696e67/calibre-library", "name": "Rating", "icon": "/icon/rating.png", "is_category": true}, {"url": "/ajax/category/736572696573/calibre-library", "name": "Series", "icon": "/icon/series.png", "is_category": true}, {"url": "/ajax/category/74616773/calibre-library", "name": "Tags", "icon": "/icon/tags.png", "is_category": true}]'
>>> # I wanted to use tags to control the syncing, so I used the "Tags" dict in the response
>>> urllib.request.urlopen(urllib.request.Request(f"{CALIBRE_SERVER_URL}/ajax/category/74616773/calibre-library", headers=headers)).read().decode()
'{"category_name": "Tags", "base_url": "/ajax/category/74616773/calibre-library", "total_num": 919, "offset": 0, "num": 100, "sort": "name", "sort_order": "asc", "subcategories": [], "items": [...]}'
>>> # Many items were returned, I happen to have hundreds of tags in the library. The one I wanted wasn't in the first 100 items (the default number to return)
>>> # I just kept increasing the offset by 100 until the response had the tag I was looking for, there was probably a faster way but I'm lazy
>>> resp = urllib.request.urlopen(urllib.request.Request(f"{CALIBRE_SERVER_URL}/ajax/category/74616773/calibre-library?offset=600", headers=headers)).read().decode()
>>> resp.find('Plato')
10292
>>> resp[10200:10400]
' "url": "/ajax/books_in/74616773/373236/calibre-library", "has_children": false}, {"name": "Plato-sync", "average_rating": 0.0, "count": 1, "url": "/ajax/books_in/74616773/31343539/calibre-library", "'
>>> # And there are my numbers in the url for the "Plato-sync" tag.
>>> # I would put 74616773 for category, and 31343539 for item
```

Depending on exactly what set of books you want synced, you would go after a different category. I have a ton of books in my library so I wanted to avoid syncing the entire thing. If that isn't an issue for you, just use the "All books" category. If you fetch the category info for that it will give you a `books_in` url that should return everything.

## TODO

1. The wifi/network setup code is a wonky, have to look at it. If you start the hook with wifi *OFF* it seems to consistently work.
