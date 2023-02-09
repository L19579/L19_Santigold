CREATE TABLE channel(
  id SERIAL PRIMARY KEY,
  external_id uuid NOT NULL,
  title TEXT NOT NULL UNIQUE,
  category TEXT NOT NULL,
  description TEXT NOT NULL,
  managing_editor TEXT NOT NULL,
  generator TEXT NOT NULL,
  -- direct link to png
  image_url TEXT NOT NULL,
  image_title TEXT NOT NULL,
  image_link TEXT NOT NULL,
  image_width INT NOT NULL,
  image_height INT NOT NULL,
  -- channel page link
  language TEXT NOT NULL,
  last_build_date TEXT NOT NULL,
  pub_date TEXT NOT NULL,
  -- channel page link
  c_link TEXT NOT NULL,
  itunes_new_feed_url TEXT NOT NULL, 
  itunes_explicit boolean NOT NULL,
  itunes_owner_name TEXT NOT NULL,
  itunes_owner_email TEXT NOT NULL,
  sy_update_period TEXT NOT NULL,
  sy_update_frequency TEXT NOT NULL
  /* 
  itunes_link (cLink), 
  itunes_category (category),
  itunes_image (image_url), 
  itunes_subtitle (description), 
  googleplay_category(category), 
  atom_Link(itunes_newFeedUrl), 
   */
);

CREATE TABLE item(
  id uuid NOT NULL,
  channel_id INT NOT NULL,
  title TEXT NOT NULL,
  -- author's email address
  author TEXT NOT NULL,
  -- TODO: drop, just == channel category
  category TEXT NOT NULL,
  description TEXT NOT NULL,
  -- repeat of description with formatting. Primary desc.
  content_encoded TEXT NOT NULL,
  -- direct link to mp3
  enclosure TEXT NOT NULL,
  -- episode page link
  i_link TEXT NOT NULL,
  pub_date TEXT NOT NULL,
  itunes_subtitle TEXT NOT NULL,
  itunes_image TEXT NOT NULL,
  itunes_duration TEXT NOT NULL,
  PRIMARY KEY (id)
  /*
  itunes_author (author), 
  itunes_summary (description), 
  */
);
