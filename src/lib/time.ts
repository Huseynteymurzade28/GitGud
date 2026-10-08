const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 31536000],
  ['month', 2592000],
  ['week', 604800],
  ['day', 86400],
  ['hour', 3600],
  ['minute', 60],
]

const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })

/** "3 days ago" style label for a Unix timestamp in seconds. */
export function relativeTime(unixSeconds: number) {
  const diff = unixSeconds - Date.now() / 1000
  for (const [unit, seconds] of UNITS) {
    if (Math.abs(diff) >= seconds)
      return rtf.format(Math.round(diff / seconds), unit)
  }
  return rtf.format(0, 'second')
}
