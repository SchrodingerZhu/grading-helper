use crate::schema::*;
use anyhow::*;
use diesel::RunQueryDsl;
use tablefy_derive::Tablefy;
use tablefy::Tablefy;
joinable!(grade -> student (student_id));
joinable!(grade -> project (project_id));

#[derive(diesel::QueryableByName,
    diesel::Queryable,
    diesel::Associations,
    diesel::Identifiable,
    Debug,
    Tablefy,
    serde::Serialize,
    serde::Deserialize)]
#[table_name="student"]
pub struct Student {
    pub id: i32,
    pub path: String,
}

#[derive(diesel::QueryableByName,
    diesel::Queryable,
    diesel::Associations,
    diesel::Identifiable,
    Debug,
    Tablefy,
    serde::Serialize,
    serde::Deserialize)]
#[table_name="project"]
pub struct Project {
    pub id: i32,
    pub path: String,
    pub manual_grade: i32,
    pub auto_grade: i32
}

#[derive(diesel::Queryable,
    diesel::Identifiable,
    diesel::Associations,
    serde::Serialize,
    Debug,
    Tablefy,
    serde::Deserialize)]
#[table_name="grade"]
#[belongs_to(Student)]
#[belongs_to(Project)]
pub struct Grade {
    pub id: i32,
    pub student_id: i32,
    pub project_id: i32,
    pub manual_grade: i32,
    pub auto_grade: i32,
    pub comment: String
}

#[derive(diesel::Queryable,
    diesel::Identifiable,
    diesel::Insertable,
    serde::Serialize,
    Debug,
    serde::Deserialize,
    Tablefy)]
#[table_name="configuration"]
pub struct Configuration {
    pub id: i32,
    pub current_student: Option<i32>,
    pub current_project: Option<i32>,
    pub auto_grade: Option<i32>,
    pub manual_grade: Option<i32>,
    pub comment: Option<String>,
    pub base_image: String
}

#[derive(Insertable, Default, Debug, AsChangeset)]
#[table_name="configuration"]
pub struct ChangeConfig<'a> {
    pub id: i32,
    pub current_student: Option<i32>,
    pub current_project: Option<i32>,
    pub auto_grade: Option<i32>,
    pub manual_grade: Option<i32>,
    pub comment: Option<&'a str>,
    pub base_image: Option<&'a str>
}

#[derive(Insertable, Default, Debug, AsChangeset)]
#[table_name="grade"]
pub struct ChangeGrade {
    pub student_id: Option<i32>,
    pub project_id: Option<i32>,
    pub manual_grade: Option<i32>,
    pub auto_grade: Option<i32>,
    pub comment: Option<String>
}

#[derive(Insertable, Default, Debug, AsChangeset)]
#[table_name="project"]
pub struct ChangeProject<'a> {
    pub path: Option<&'a str>,
    pub manual_grade: Option<i32>,
    pub auto_grade: Option<i32>
}

#[derive(Insertable, Default, Debug, AsChangeset)]
#[table_name="student"]
pub struct ChangeStudent<'a> {
    pub path: Option<&'a str>
}

impl Configuration {
    pub fn initialize(conn: &diesel::SqliteConnection, base_image: &str) -> Result<()> {
        use crate::schema::configuration::table;
        diesel::insert_into(table)
            .values(&ChangeConfig{
                id: 1,
                current_student: None,
                current_project: None,
                auto_grade: None,
                manual_grade: None,
                comment: None,
                base_image: Some(base_image)
            })
            .execute(conn)?;
        Ok(())
    }
    pub fn get_global(conn: &diesel::SqliteConnection) -> Result<Self> {
        use crate::schema::configuration::dsl::*;
        configuration
            .first::<Configuration>(conn)
            .map_err(Into::into)
    }
    pub fn store(&self, conn :&diesel::SqliteConnection) -> Result<usize> {
        diesel::replace_into(crate::schema::configuration::table)
            .values(self)
            .execute(conn)
            .map_err(Into::into)
    }
}