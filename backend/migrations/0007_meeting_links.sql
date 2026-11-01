-- Live class links (Zoom, Meet, Teams, ...). A weekly schedule slot carries the default link of
-- that class; a lesson may override it for its own session.

ALTER TABLE course_schedule ADD COLUMN meeting_url TEXT;
ALTER TABLE lessons ADD COLUMN meeting_url TEXT;
