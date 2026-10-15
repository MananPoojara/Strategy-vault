export function Icon({ name }) {
  if (name === 'user') {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="8" r="3.25" />
        <path d="M5.75 19c.65-3.25 2.82-5 6.25-5s5.6 1.75 6.25 5" />
      </svg>
    )
  }

  if (name === 'lock') {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="5.75" y="10" width="12.5" height="9" rx="2.5" />
        <path d="M8.75 10V7.6A3.25 3.25 0 0 1 12 4.35a3.25 3.25 0 0 1 3.25 3.25V10" />
      </svg>
    )
  }

  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3.75 12s3-5.25 8.25-5.25S20.25 12 20.25 12 17.25 17.25 12 17.25 3.75 12 3.75 12Z" />
      <circle cx="12" cy="12" r="2.25" />
    </svg>
  )
}

