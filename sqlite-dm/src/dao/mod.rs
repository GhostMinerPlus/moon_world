use std::io::{self, Error, ErrorKind};

use edge_lib::util::Path;
use sqlx::{Sqlite, Pool, Row};

mod main {
    use std::io;

    use edge_lib::util::Step;
    use sqlx::{Sqlite, Pool};

    pub async fn delete_edge_with_source_code(
        pool: Pool<Sqlite>,
        source: &str,
        paper: &str,
        code: &str,
    ) -> io::Result<()> {
        sqlx::query("delete from edge_t where source = ? and paper = ? and code = ?")
            .bind(source)
            .bind(paper)
            .bind(code)
            .execute(&pool)
            .await
            .map_err(|e| io::Error::other(e))?;
        Ok(())
    }

    pub fn gen_sql_stm(first_step: &Step, step_v: &[Step]) -> String {
        let sql = if first_step.arrow == "->" {
            format!(
            "select v_{}.root from (select target as root, id from edge_t where source=? and paper=? and code=?) v_0",
            step_v.len(),
       )
        } else {
            format!(
            "select v_{}.root from (select source as root, id from edge_t where target=? and paper=? and code=?) v_0",
            step_v.len(),
       )
        };
        let mut root = format!("v_0");
        let mut no = 0;
        let join_v = step_v.iter().map(|step| {
            let p_root = root.clone();
            no += 1;
            root = format!("v_{no}");
            if step.arrow == "->" {
                format!(
                    "join (select target as root, source, id from edge_t where paper=? and code=?) v_{no} on v_{no}.source = {p_root}.root",
               )
            } else {
                format!(
                    "join (select source as root, target, id from edge_t where paper=? and code=?) v_{no} on v_{no}.source = {p_root}.root",
               )
            }
        }).reduce(|acc, item| {
            format!("{acc}\n{item}")
        }).unwrap_or_default();
        format!("{sql}\n{join_v} order by v_{}.id", step_v.len())
    }

    #[cfg(test)]
    mod test_gen_sql {
        use edge_lib::util::Step;

        #[test]
        fn test_gen_sql() {
            let sql = super::gen_sql_stm(
                &Step {
                    arrow: "->".to_string(),
                    code: "code".to_string(),
                    paper: "".to_string(),
                },
                &vec![Step {
                    arrow: "->".to_string(),
                    code: "code".to_string(),
                    paper: "".to_string(),
                }],
            );
            println!("{sql}")
        }
    }
}

pub async fn insert_edge(
    pool: Pool<Sqlite>,
    source: &str,
    paper: &str,
    code: &str,
    target_v: &Vec<String>,
) -> io::Result<()> {
    if target_v.is_empty() {
        return Ok(());
    }
    log::info!("commit target_v: {}", target_v.len());
    let value_v = target_v
        .iter()
        .map(|_| format!("(?,?,?,?)"))
        .reduce(|acc, item| {
            if acc.is_empty() {
                item
            } else {
                format!("{acc},{item}")
            }
        })
        .unwrap();

    let sql = format!("insert into edge_t (source,paper,code,target) values {value_v}");
    let mut statement = sqlx::query(&sql);
    for target in target_v {
        statement = statement.bind(source).bind(paper).bind(code).bind(target);
    }
    statement
        .execute(&pool)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    Ok(())
}

pub async fn get(pool: Pool<Sqlite>, path: &Path) -> io::Result<Vec<String>> {
    let first_step = &path.step_v[0];
    let sql = main::gen_sql_stm(first_step, &path.step_v[1..]);
    let mut stm = sqlx::query(&sql).bind(path.root_op.as_ref().unwrap());
    for step in &path.step_v {
        stm = stm.bind(&step.paper).bind(&step.code);
    }
    let rs = stm
        .fetch_all(&pool)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let mut arr = Vec::new();
    for row in rs {
        arr.push(row.get(0));
    }
    Ok(arr)
}

pub async fn delete_edge_with_source_code(
    pool: Pool<Sqlite>,
    paper: &str,
    source: &str,
    code: &str,
) -> io::Result<()> {
    main::delete_edge_with_source_code(pool, source, paper, code).await
}

pub async fn clear(pool: Pool<Sqlite>) -> io::Result<()> {
    sqlx::query("delete from edge_t where 1 = 1")
        .execute(&pool)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    Ok(())
}

pub async fn clear_paper(pool: Pool<Sqlite>, paper: &str) -> io::Result<()> {
    sqlx::query("delete from edge_t where paper = ?")
        .bind(paper)
        .execute(&pool)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    Ok(())
}
