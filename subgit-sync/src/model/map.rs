use git2::Oid;
use hex;
use rusqlite::Connection;
use chrono::Utc;

pub struct CommitMapper<'a> {
    pub conn: &'a Connection,
}

impl<'a> CommitMapper<'a> {
    pub fn get_translated(
        &self,
        maybe_sha: Option<&Oid>,
        source: super::Location,
    ) -> Option<Oid> {
        let sha = match maybe_sha {
            Some(sha_) => sha_,
            None => return None,
        };
//        let partial_path = sha_path(sha);
//        let full_path = format!("{}_to_{}/{}", source.as_ref(), dest.as_ref(), partial_path);
//        let content = fs::content_of_file_if_exists(
//            &fs::make_absolute(&self.workdir()).unwrap().join(full_path),
//        );
//        content.map(|oid_str| {
//            Oid::from_bytes(&hex::decode(oid_str).unwrap())
//                .expect("The format should be correct for a stored sha")
//        })

        let mut stmt = self.conn.prepare(&format!(r#"
            SELECT dest
            FROM {}
            WHERE :source = source
            ORDER BY timestamp DESC
        "#, source.as_source_table())).unwrap();
        let mut rows = stmt.query_named(&[
            (":source", &format!("{}", sha)),
        ]).unwrap();
        if let Some(row) = rows.next() {
            let row = row.unwrap();
            let value: String = row.get(0);
            Some(Oid::from_bytes(&hex::decode(value.as_bytes()).unwrap())
                .expect("The format should be correct for a stored sha"))
        } else {
            None
        }
    }

    pub fn has_sha(
        &self,
        sha: &Oid,
        source: super::Location,
    ) -> bool {
        self.get_translated(Some(sha), source).is_some()
    }

    pub fn set_translated(
        &self,
        sha: &Oid,
        source: super::Location,
        translated: &Oid,
    ) {
        self.conn.execute_named(
            &format!(r#"
                    INSERT INTO {} (source, dest, timestamp)
                    VALUES (:source, :dest, :timestamp)
                "#, source.as_source_table()),
            &[(":source", &format!("{}", sha)), (":dest", &format!("{}", translated)), (":timestamp", &Utc::now())],
        ).unwrap();
    }
}
