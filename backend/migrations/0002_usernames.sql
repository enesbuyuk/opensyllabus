-- Optional login name: sign-in accepts the email address or the username.
-- Usernames are 3-32 characters of a-z, 0-9, ".", "_" and "-" and never contain "@".

ALTER TABLE users ADD COLUMN username TEXT COLLATE NOCASE;

CREATE UNIQUE INDEX users_username_unique ON users (username) WHERE username IS NOT NULL;

-- Existing accounts get the part of their email before "@" when it is a valid
-- username and no other account shares it; the rest can set one in Settings.
UPDATE users
SET username = lower(substr(email, 1, instr(email, '@') - 1))
WHERE length(substr(email, 1, instr(email, '@') - 1)) BETWEEN 3 AND 32
  AND lower(substr(email, 1, instr(email, '@') - 1)) NOT GLOB '*[^a-z0-9._-]*'
  AND lower(substr(email, 1, 1)) GLOB '[a-z0-9]'
  AND (
      SELECT count(*) FROM users AS other
      WHERE lower(substr(other.email, 1, instr(other.email, '@') - 1)) = lower(substr(users.email, 1, instr(users.email, '@') - 1))
  ) = 1;
