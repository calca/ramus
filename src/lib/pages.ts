// Convenzioni delle pagine lato frontend. `slugify` deve rispecchiare
// esattamente `vault::slugify` in ramus-core: è quella che decide a quale
// file risolve un [[link]], sia in autocomplete sia al click.

export function slugify(name: string): string {
  return name
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .join("-");
}
