# Diary Generator

Generate a web diary from a [Notion database](https://www.notion.so/guides/creating-a-database)
with very little configuration!

This is a more opinionated sibling project to [Notion Generator](https://github.com/Mathspy/notion-generator).
While Notion Generator is able to generate HTML pages from any Notion document, diary generator
expect a database of documents structured in a very specific way described below and it will generate a diary from it.

A "web diary" here refers to a website that looks like a diary or a journal where days are tagged with a specific date. If you ever watched or read one of those stories where they go like "Day 74: the storm has shipwrecked half of the fleet..." and you thought to yourself "I WANT TO MAKE SOMETHING LIKE THIS SO BAD AHHHH." Then look no further, friend!

## Usage

1. Start by [creating a database in Notion](https://www.notion.so/guides/creating-a-database) also [create an internal integration](https://www.notion.so/my-integrations). Their names don't matter, feel free to name them something that reminds you of their purpose.
2. Share your database with your integration by clicking the share menu and then the invite box, your integration should appear for selection.
3. Rename the main field in your database to `name` and create `date`, `published` fields with type Date and `url`, `description` fields with type Text. All of these names are case-sensitive.
4. Start writing! Each entry in the database should have either one of `date` OR `url` fields filled (NOT both). Having the `date` field turns it into the date's entry. Having the `url` field turns it into an article accessible from `/{url}`.\
`description` gives the entry or article a description. And finally `published` gives the entry or article a date to be published at. (Before that date it will be automatically skipped)
5. Once you're ready to publish copy your database's ID and pass it as the only argument, (i.e `./diary-generator 6e0eb85f60474efba1304f92d2abfa2c`) and make sure `NOTION_TOKEN` env variable is set to your integration's secret.
6. Your diary will be generated into `output/` directory and you can do whatever you want with it!

You can arguably avoid the `date` field and have a normal blog-like website but that's a bit boring and more people should write public diaries!\
For inspiration check my [Game Dev Diary](https://gamediary.dev) where I write about my adventures learning game development!

## Advanced features
### Independent pages
If you create a `pages/` directory in the folder where you handle generation filled with partial HTML files (the main content of the page without the layout AKA head, headers, or footers). Those pages will be automatically copied over to `output/` and wrapped in the layout.

The difference between these pages and pages in Notion with `url` is that these don't count as articles and won't be listed in the `/articles` page. This are useful for pages like `/404.html`.

### `public/` directory for assets
If you create a `public/` directory in the folder where you handle generation all its content will be copied over to `output/`

### `config.json` for configuring your diary
You can also include a `config.json` directory in the directory to modify the behavior of the generator. Currently supported fields are:
```js
{
  // The name of your diary which will automatically be used in page titles
  "name": String
}
```

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in diary generator by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
