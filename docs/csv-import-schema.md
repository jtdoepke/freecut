# CSV Import Schema

Freecut imports cut-list CSV files with a required header row. The import is deliberately small and dependency-free.

## Encoding and separators

- UTF-8 text.
- Comma-separated fields.
- First non-empty line is the header.
- Quoted fields are supported with `"..."`.
- Escaped quotes inside quoted fields use doubled quotes: `""`.
- Multiline quoted fields are not supported yet.

## Required columns

The header must contain these logical columns:

| Logical column | Accepted header names | Meaning |
| --- | --- | --- |
| `label` | `label`, `name` | Cut-piece label/name. Required for cut rows. Ignored for stock rows. |
| `width` | `width` | Piece width as a positive number in the current project unit. Decimal (`12.5`, `100,5`) and fraction (`1/2`, `3 1/4`) notation are accepted; values are kept to three decimal places. |
| `length` | `length` | Piece length as a positive number in the current project unit, same notation as `width`. |
| `quantity` | `quantity`, `amount` | Positive integer quantity. |

## Optional columns

| Logical column | Accepted header names | Values | Default |
| --- | --- | --- | --- |
| `pattern` | `pattern` | `none`, `width`, `length`, `parallel_width`, `parallel_length`, `parallel to width`, `parallel to length` | `none` |
| `rotation` | `rotation`, `can_rotate` | `true`, `false`, `yes`, `no`, `1`, `0`, `y`, `n` | `true` for cut rows |
| `piece_type` | `piece_type`, `type`, `kind` | `cut`, `cutpiece`, `stock`, `stockpiece` | `cut` |

## Row behavior

- `piece_type` omitted or empty means `cut`.
- `cut` rows create `domain::CutPiece` values.
- `stock` rows create finite `domain::StockPiece` values with `quantity: Some(quantity)`.
- Valid rows are imported even if other rows contain errors.
- Errors are reported per row.
- Imported pieces receive new `PieceId`s after the current maximum project ID.
- Imported dimensions are interpreted in the current project unit (`CutSettings::unit`) and kept to three decimal places; unit conversion is not part of this import phase.

## Example

```csv
label,width,length,quantity,pattern,rotation,piece_type
side panel,700,500,2,width,true,cut
shelf,600.5,300,4,none,true,cut
trim,1/2,1 1/4,8,none,true,cut
birch sheet,2440,1220,3,length,,stock
```
