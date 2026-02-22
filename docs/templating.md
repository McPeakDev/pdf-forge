# pdf-forge Templating Guide

pdf-forge converts HTML fragments to PDF using a subset of HTML elements and
a Tailwind-inspired class system. This guide covers everything you can use in
your templates.

---

## Page layout

| Default       | Value                      |
| ------------- | -------------------------- |
| Page size     | A4 portrait (595 × 842 pt) |
| Page margins  | 40 pt on all sides         |
| Content width | 515 pt                     |

Pass `--landscape` on the CLI (or `PageOrientation::Landscape` in code) to
swap the dimensions to 842 × 595 pt.

---

## Page breaks

Use the `.page` or `.page-break` class on any block element to force a page
break **after** that element:

```html
<!-- everything above this line goes on page 1 -->
<div class="page"></div>
<!-- content below starts on page 2 -->

<div class="page-break"></div>
```

The same effect can be achieved with a CSS inline style:

```html
<div style="break-after: page">…</div>
<div style="page-break-after: page">…</div>
<div style="page-break-after: always">…</div>
```

To force a break **before** an element instead:

```html
<div style="break-before: page">…</div>
<div style="page-break-before: always">…</div>
```

To prevent a block from splitting across pages (keeps the whole element
together on one page):

```html
<div class="break-inside-avoid">…</div>
<div style="page-break-inside: avoid">…</div>
```

---

## Supported HTML elements

| Element                           | Notes                                                |
| --------------------------------- | ---------------------------------------------------- |
| `<h1>` – `<h3>`                   | Block headings                                       |
| `<p>`                             | Paragraph                                            |
| `<div>`                           | Generic block / flex container                       |
| `<span>`                          | Inline text wrapper                                  |
| `<ul>`, `<ol>`                    | Unordered / ordered list                             |
| `<li>`                            | List item – bullet (•) or number added automatically |
| `<table>`, `<tr>`, `<td>`, `<th>` | Table; rows split across pages automatically         |
| `<img>`                           | Image – **must** use a base64 data URI (see below)   |

Unknown elements are silently ignored (treated as `display: none`).

---

## Images

Only inline **base64 data URIs** are supported. File paths and `http://` URLs
are rejected with an error.

```html
<img
  src="data:image/png;base64,iVBORw0KGgoAAAA..."
  style="width: 120px; height: 80px"
/>
```

Supported formats: PNG, JPEG.

---

## Tailwind-style utility classes

### Spacing

```
p-{n}   pt-{n}  pr-{n}  pb-{n}  pl-{n}   — padding (n × 4 pt)
m-{n}   mt-{n}  mr-{n}  mb-{n}  ml-{n}   — margin  (n × 4 pt)
```

Examples: `p-4` = 16 pt all sides, `mt-2` = 8 pt top margin.

### Typography

| Class         | Effect                    |
| ------------- | ------------------------- |
| `text-xs`     | 10 pt                     |
| `text-sm`     | 12 pt                     |
| `text-base`   | 14 pt (default)           |
| `text-lg`     | 16 pt                     |
| `text-xl`     | 20 pt                     |
| `text-2xl`    | 24 pt                     |
| `text-3xl`    | 30 pt                     |
| `font-bold`   | Bold weight               |
| `font-normal` | Normal weight             |
| `italic`      | Italic style              |
| `underline`   | Underline decoration      |
| `text-left`   | Left-align text (default) |
| `text-center` | Centre-align text         |
| `text-right`  | Right-align text          |

### Colour

Named colours (Tailwind palette subset):

```
text-{colour}   bg-{colour}
```

Supported colours: `gray-100/200/300/400/500/600`, `red-500`, `green-500`,
`blue-500`, `yellow-500`, `white`, `black`.

### Width

| Class    | Effect         |
| -------- | -------------- |
| `w-full` | 100% of parent |
| `w-1/2`  | 50%            |
| `w-1/3`  | 33 %           |
| `w-2/3`  | 66 %           |
| `w-1/4`  | 25%            |
| `w-3/4`  | 75%            |
| `w-{n}`  | n × 4 pt       |

### Flexbox

| Class             | Effect                               |
| ----------------- | ------------------------------------ |
| `flex`            | `display: flex` (row direction)      |
| `flex-col`        | Flex, column direction               |
| `flex-1`          | `flex-grow: 1; flex-shrink: 1`       |
| `flex-wrap`       | Allow wrapping                       |
| `items-center`    | `align-items: center`                |
| `items-start`     | `align-items: flex-start`            |
| `items-end`       | `align-items: flex-end`              |
| `justify-center`  | `justify-content: center`            |
| `justify-between` | `justify-content: space-between`     |
| `justify-around`  | `justify-content: space-around`      |
| `justify-evenly`  | `justify-content: space-evenly`      |
| `gap-{n}`         | Gap between flex children (n × 4 pt) |

### Page-break helpers

| Class                | Effect                                      |
| -------------------- | ------------------------------------------- |
| `page`               | Page break **after** this element           |
| `page-break`         | Page break **after** this element           |
| `break-after`        | Page break **after** this element           |
| `break-before`       | Page break **before** this element          |
| `break-inside-avoid` | Keep element intact (no split across pages) |

---

## Inline styles

Inline `style=""` attributes support a subset of CSS properties:

| Property                          | Accepted values                 |
| --------------------------------- | ------------------------------- |
| `color`                           | `#rrggbb`, `#rgb`, `rgb(r,g,b)` |
| `background-color`                | same as `color`                 |
| `font-size`                       | `{n}px`, `{n}pt`, `{n}rem`      |
| `font-weight`                     | `bold`, `700`, `normal`, `400`  |
| `font-style`                      | `italic`, `normal`              |
| `text-decoration`                 | `underline`, `none`             |
| `text-align`                      | `left`, `center`, `right`       |
| `width` / `height`                | `{n}px`, `{n}%`, `{n}pt`        |
| `margin[-top/right/bottom/left]`  | `{n}px`, `{n}pt`                |
| `padding[-top/right/bottom/left]` | `{n}px`, `{n}pt`                |
| `border-width`                    | `{n}px`                         |
| `gap`                             | `{n}px`                         |
| `break-after`                     | `page`, `always`                |
| `break-before`                    | `page`, `always`                |
| `page-break-after`                | `page`, `always`                |
| `page-break-before`               | `page`, `always`                |
| `page-break-inside`               | `avoid`                         |

---

## Full example

```html
<div class="p-6">
  <h1 class="text-3xl font-bold mb-4">Report Title</h1>

  <p class="mb-4">
    First section content with <span class="font-bold">bold</span> and
    <span class="italic">italic</span> words.
  </p>

  <ul class="mb-4">
    <li>Bullet one</li>
    <li>Bullet two</li>
  </ul>

  <!-- Force the next section onto a new page -->
  <div class="page"></div>

  <h2 class="text-2xl font-bold mb-2">Second Section</h2>
  <table class="w-full">
    <tr>
      <th class="p-2 bg-gray-200">Column A</th>
      <th class="p-2 bg-gray-200">Column B</th>
    </tr>
    <tr>
      <td class="p-2">Value 1</td>
      <td class="p-2">Value 2</td>
    </tr>
  </table>
</div>
```
