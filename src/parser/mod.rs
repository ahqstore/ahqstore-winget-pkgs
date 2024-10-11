use ahqstore_types::AHQStoreApplication;
use serde_yml::from_str;
use std::{
  cmp::Ordering,
  fs::{self, File},
  io::Write,
};
use version_compare::Version;

struct Map {
  entries: usize,
  files: usize,
  c_file: File,
  search: File,
}

impl Map {
  fn new() -> Self {
    let _ = fs::create_dir_all("./db/map");
    let _ = fs::create_dir_all("./db/search");
    let _ = fs::create_dir_all("./db/apps");
    let _ = fs::create_dir_all("./db/dev");
    let _ = fs::create_dir_all("./db/res");

    let mut file = File::create("./db/map/1.json").unwrap();
    let _ = file.write(b"{");

    let mut search = File::create("./db/search/1.json").unwrap();
    let _ = search.write(b"[");

    Self {
      entries: 0,
      files: 1,
      c_file: file,
      search,
    }
  }

  fn close_file(&mut self) {
    let _ = self.search.write_all(b"]");
    let _ = self.search.flush();
    let _ = self.c_file.write_all(b"}");
    let _ = self.c_file.flush();
  }

  fn new_file(&mut self) {
    self.files += 1;
    self.entries = 0;
    self.close_file();

    let mut map = File::create("./db/map/1.json").unwrap();
    let _ = map.write(b"{");

    let mut search = File::create("./db/map/1.json").unwrap();
    let _ = search.write(b"[");

    self.c_file = map;
    self.search = search;
  }

  fn add_author(&mut self, author: &str, app_id: &str) {
    let file = format!("./db/dev/{}", author);
    let mut val = fs::read_to_string(&file).unwrap_or("".to_string());
    val.push_str(&format!("{}\n", &app_id));

    let _ = fs::write(&file, val);
  }

  fn add(&mut self, app: AHQStoreApplication) {
    if self.entries >= 100_000 {
      self.new_file();
    }
    println!("{}", self.entries);
    if self.entries > 0 {
      let _ = self.c_file.write(b",");
      let _ = self.search.write(b",");
    }

    self.add_author(&app.authorId, &app.appId);
    self.entries += 1;

    let _ = self
      .c_file
      .write(format!("\"{}\":\"{}\"", app.appDisplayName, app.appId).as_bytes());
    let _ = self.search.write(
      format!(
        "{{\"name:\": {:?}, \"title\": {:?}, \"id\": {:?}}}",
        app.appDisplayName, app.appShortcutName, app.appId
      )
      .as_bytes(),
    );

    let (app_str, res) = app.export();

    let _ = fs::write(format!("./db/apps/{}.json", &app.appId), app_str);

    let _ = fs::create_dir_all(format!("./db/res/{}", &app.appId));

    for (id, bytes) in res {
      let _ = fs::write(format!("./db/res/{}/{}", &app.appId, id), bytes);
    }
  }

  fn finish(mut self) {
    self.close_file();

    let _ = fs::write("./db/total", self.files.to_string());
  }
}

pub fn parser() {
  println!("⏲️ Please wait...");
  let _ = fs::remove_dir_all("./db");
  let _ = fs::create_dir_all("./db");

  let _ = fs::copy("./home.json", "./db/home.json");

  let mut map = Map::new();

  for letter in fs::read_dir("./winget-pkgs/manifests").unwrap() {
    let letter = letter.unwrap().file_name();
    let letter = letter.to_str().unwrap();

    for author in fs::read_dir(format!("./winget-pkgs/manifests/{}", &letter)).unwrap() {
      let author = author.unwrap().file_name();
      let author = author.to_str().unwrap();

      app_parse(letter, author, &mut map);
    }
  }
  map.finish();
  println!("✅ Done!");
}

fn app_parse(letter: &str, author: &str, map: &mut Map) {
  for app in fs::read_dir(format!("./winget-pkgs/manifests/{}/{}", &letter, &author)).unwrap() {
    let app = app.unwrap();

    if !app.file_type().unwrap().is_dir() {
      continue;
    }

    let app = app.file_name();

    let app = app.to_str().unwrap();

    if app == ".validation" {
      continue
    }

    let inside = fs::read_dir(format!(
      "./winget-pkgs/manifests/{}/{}/{}",
      &letter, &author, &app
    ))
    .unwrap()
    .into_iter();

    let inside = inside.map(|x| x.unwrap()).filter(|x| x.file_type().unwrap().is_dir()).map(|x| x.file_name()).collect::<Vec<_>>();
    let inside = inside.into_iter();
    let inside = inside.filter(|x| x != ".validation").collect::<Vec<_>>();
    let inside = inside.into_iter();

    let mut versions = inside
      .clone()
      .filter(|x| Version::from(x.to_str().unwrap_or("unknown")).is_some())
      .collect::<Vec<_>>();

    versions.sort_by(|x, y| {
      let (x, y) = (x.to_str().unwrap_or("0.0.0"), y.to_str().unwrap_or("0.0.0"));
      let x = Version::from(x).unwrap();
      let y = Version::from(y).unwrap();

      if x == y {
        Ordering::Equal
      } else if x > y {
        Ordering::Greater
      } else {
        Ordering::Less
      }
    });

    if !versions.is_empty() {
      let _v = versions.pop().unwrap();
      drop(versions);

      //println!("Author: {author} App: {app} Ver: {v:?}");
    }
    
    for product in inside.filter(|x| Version::from(x.to_str().unwrap_or("unknown")).is_none()) {
      if product.to_str().unwrap_or(".yaml").ends_with(".yaml") {
        continue
      }

      app_parse(
        letter,
        &format!("{author}/{app}/{}", product.to_str().unwrap_or("unknown")),
        map,
      );
    }
    
  }
}
