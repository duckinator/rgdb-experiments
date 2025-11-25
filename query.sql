SELECT
  versions.created_at,
  rubygems.name,
  versions.canonical_number
FROM versions LEFT OUTER JOIN rubygems ON versions.rubygem_id = rubygems.id
WHERE versions.created_at BETWEEN date_subtract(CURRENT_TIMESTAMP, '1 week'::interval, 'UTC') AND CURRENT_TIMESTAMP
ORDER BY versions.created_at DESC
LIMIT 10000
