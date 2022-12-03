mod database;
use std::ops::Deref;

use database::prelude::*;
use salvo::http::request;
use salvo::prelude::*;
use sea_orm::{Database, DatabaseBackend, DatabaseConnection, EntityTrait, JsonValue, Statement};
use serde_json::{json, Map};

use salvo::session::{CookieStore, Session, SessionDepotExt, SessionHandler};

use self::database::{article_tb, comment_tb, tag_tb, user_tb, view_tb};

use sea_orm::{entity::*, query::*};

use salvo::http::StatusCode;

use tera::{Context, Tera};
use time::{Duration, OffsetDateTime};

use jsonwebtoken::{self, EncodingKey};

use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub user_id: String,
    pub exp: i64,
}

pub const SECRET_KEY: &str = "130ae5ac67bb6c7ecfe1f2924076964b130ae5ac67bb6c7ecfe1f2924076964b";

const ResponseTextForError: u8 = 1;
const ResponseJsonForError: u8 = 2;

#[derive(Debug)]
pub struct UniformError<const ERRORCODE: u8 = 1>(anyhow::Error);

impl<const ERRORCODE: u8, T: Into<anyhow::Error>> From<T> for UniformError<ERRORCODE> {
    fn from(v: T) -> Self {
        UniformError(v.into())
    }
}

#[async_trait]
impl<const ERRORCODE: u8> Writer for UniformError<ERRORCODE> {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        if ERRORCODE == 1 {
            res.with_status_code(StatusCode::BAD_REQUEST)
                .render(Text::Plain(self.0.to_string()));
        } else {
            let r = json!({
                "code":400,
                "msg":self.0.to_string()
            });
            res.with_status_code(StatusCode::BAD_REQUEST)
                .render(Text::Json(r.to_string()));
        }
    }
}

pub trait ConverOptionToResult<T> {
    fn to_result<const ERRORCODE: u8>(self) -> Result<T, UniformError<ERRORCODE>>;
}

impl<T> ConverOptionToResult<T> for Option<T> {
    fn to_result<const ERRORCODE: u8>(self) -> Result<T, UniformError<ERRORCODE>> {
        match self {
            Some(x) => Ok(x),
            None => Err(UniformError(anyhow::anyhow!("empty data"))),
        }
    }
}
/// Return UserTb::Model, TagTb::Model, view_count, comment_count
async fn getRelativeInformationFromArticle(
    article_id: u64,
    user_id: u64,
    tag_id: u64,
    db: &DatabaseConnection,
) -> Result<(user_tb::Model, tag_tb::Model, u64, u64), UniformError> {
    let tag = TagTb::find_by_id(tag_id as i32)
        .one(db)
        .await?
        .to_result()?;
    let user = UserTb::find_by_id(user_id as i32)
        .one(db)
        .await?
        .to_result()?;
    let count = ViewTb::find()
        .filter(view_tb::Column::ArticleId.eq(article_id))
        .count(db)
        .await?;
    let comment_count = CommentTb::find()
        .filter(comment_tb::Column::ArticleId.eq(article_id))
        .count(db)
        .await?;
    Ok((user, tag, count, comment_count))
}

async fn getHotArticleList(db: &DatabaseConnection) -> Result<Vec<JsonValue>, UniformError> {
    let r = ViewTb::find()
        .from_raw_sql(Statement::from_string(
            DatabaseBackend::MySql,
            String::from(
                r#"
		SELECT
		    T.title , T.id , T.Counts
			FROM
				(
				SELECT
					a.id,
					a.title,
					COUNT( c.id ) AS Counts 
				FROM
					article_tb a
					LEFT JOIN view_tb c ON a.id = c.article_id 
				GROUP BY
					a.id 
				) AS T 
			ORDER BY 
			T.Counts DESC LIMIT 8;
	"#,
            ),
        ))
        .into_json()
        .all(db)
        .await?;
    Ok(r)
}

async fn getPersonRightState<const I: u8>(
    user_id: i32,
    db: &DatabaseConnection,
) -> Result<(user_tb::Model, u64), UniformError<I>> {
    let info = UserTb::find_by_id(user_id).one(db).await?.to_result()?;
    let post_count = ArticleTb::find()
        .filter(article_tb::Column::UserId.eq(user_id))
        .count(db)
        .await?;
    Ok((info, post_count))
}

#[handler]
pub async fn home(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let db = depot.get::<DatabaseConnection>("db_conn").to_result()?;
    let page = match req.param::<u64>("page") {
        Some(x) => x - 1,
        _ => 0,
    };
    let pagination = ArticleTb::find()
        .order_by_desc(article_tb::Column::UpdateTime).filter(article_tb::Column::ArticleState.eq(1))
        .into_json()
        .paginate(db, 10);
    let mut data = pagination.fetch_page(page).await?;
    let tera = depot.get::<Tera>("tera").to_result()?;
    for model in &mut data {
        let id = model.get("id").to_result()?.as_u64().to_result()?;
        let tag_id = model.get("tag_id").to_result()?.as_u64().to_result()?;
        let user_id = model.get("user_id").to_result()?.as_u64().to_result()?;
        let result = getRelativeInformationFromArticle(id, user_id, tag_id, db).await?;
        model["userName"] = json!(result.0.name);
        model["tagName"] = json!(result.1.name);
        model["read_count"] = json!(result.2);
        model["commentCount"] = json!(result.3);
    }

    let hot_list = getHotArticleList(db).await?;
    //println!("{data:?}");
    let mut context = Context::new();
    let base_url = depot.get::<String>("base_url").to_result()?;
    let total_page = ArticleTb::find().count(db).await?;
    let login_data = {
        match depot.jwt_auth_state() {
            JwtAuthState::Authorized => {
                let data = depot.jwt_auth_data::<JwtClaims>().to_result()?;
                let info =
                    getPersonRightState(i32::from_str_radix(data.claims.user_id.as_str(), 10)?, db)
                        .await?;
                let avatar = info.0.avatar.unwrap_or_default();
                let username = info.0.name.unwrap_or_default();
                let level = info.0.privilege.unwrap_or_default();
                let post_count = info.1;
                json!({
                    "login":true,
                    "avatar":avatar,
                    "name":username,
                    "level":level,
                    "post_count":post_count
                })
            }
            _ => {
                //println!("no login");
                json!({
                    "login":false,
					"avatar":""
                })
            }
        }
    };
    context.insert("baseUrl", base_url);
    context.insert("login", &login_data);
    context.insert("articles", &data);
    context.insert("total", &total_page);
    context.insert("commentCount", &10);
    context.insert("page", &page);
    context.insert("hotArticles", &hot_list);
    let r = tera.render("home.html", &context)?;
    res.render(Text::Html(r));
    Ok(())
}

#[handler]
pub async fn login(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<ResponseJsonForError>> {
    let name = req.form::<String>("nickName").await.to_result()?;
    let pass = req.form::<String>("password").await.to_result()?;
    let pass = md5::compute(pass);
    let pass = format!("{:?}", pass);
    let db = depot.get::<DatabaseConnection>("db_conn").to_result()?;
    let base_url = depot.get::<String>("base_url").to_result()?;
    let Some(r) = UserTb::find().filter(user_tb::Column::Name.eq(name.clone())).filter(user_tb::Column::Password.eq(pass)).one(db).await? else{
		println!("no data found");
		let r = json!({
			"code":400,
			"msg": "用户名或密码错误",
			"baseUrl":base_url
		 });
		 res.render(Text::Json(r.to_string()));
		 return Ok(())
	};
    let exp = OffsetDateTime::now_utc() + Duration::days(1);
    let user_id = r.id;
    let claim = JwtClaims {
        user_id: user_id.to_string(),
        exp: exp.unix_timestamp(),
    };
    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claim,
        &EncodingKey::from_secret(SECRET_KEY.as_bytes()),
    )?;
    let r = json!({
       "code":200,
       "msg": "登录成功",
       "token": token,
       "baseUrl":base_url
    });
    res.render(Text::Json(r.to_string()));
    Ok(())
}

#[handler]
pub async fn person_list(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let page = req.param::<u64>("page").unwrap_or(1);
    let page = page - 1;
    let data = depot.jwt_auth_data::<JwtClaims>().to_result()?;
    let user_id = data.claims.user_id.clone();
    let db = depot.get::<DatabaseConnection>("db_conn").to_result()?;
    let sql = format!(
        r#"SELECT
	R.AID,
	R.`name`,
	R.title,
	R.view_count,
	R.update_time,
	R.article_state,
	COUNT( comment_tb.id ) AS comment_count 
FROM
	(
	SELECT
		tag_tb.`name`,
		article_tb.title,
		article_tb.id AS AID,
		article_tb.update_time,
		article_tb.article_state,
		COUNT( view_tb.article_id ) AS view_count 
	FROM
		article_tb
		LEFT JOIN tag_tb ON tag_tb.id = article_tb.tag_id
		LEFT JOIN view_tb ON view_tb.article_id = article_tb.id 
	WHERE
		article_tb.user_id = {user_id}
	GROUP BY
		AID 
	) AS R
	LEFT JOIN comment_tb ON comment_tb.article_id = R.AID 
GROUP BY
	AID 
ORDER BY
	R.update_time DESC
	LIMIT {page}, 10"#
    );
    let r = ArticleTb::find()
        .from_raw_sql(Statement::from_string(DatabaseBackend::MySql, sql))
        .into_json()
        .all(db)
        .await?;
    let info = getPersonRightState(i32::from_str_radix(user_id.as_str(), 10)?, db).await?;
    let avatar = info.0.avatar.unwrap_or_default();
    let username = info.0.name.unwrap_or_default();
    let level = info.0.privilege.unwrap_or_default();
    let post_count = info.1;
    let login_v = json!({
        "login":true,
        "avatar":avatar,
        "name":username,
        "level":level,
        "post_count":post_count
    });
    let tera = depot.get::<Tera>("tera").to_result()?;
	let total_count = ArticleTb::find().filter(article_tb::Column::UserId.eq(user_id)).count(db).await?;
    let mut context = Context::new();
	let base_url = depot.get::<String>("base_url").to_result()?;
	let hot_list = getHotArticleList(db).await?;
    context.insert("articles", &r);
	context.insert("login",&login_v);
	context.insert("page", &(page+1));
	context.insert("total",&total_count);
	context.insert("baseUrl",&base_url);
	context.insert("hotArticles",&hot_list);
	//println!("{context:#?}");
    let r = tera.render("list.html", &context)?;
	res.render(Text::Html(r));
	Ok(())
}

#[handler]
pub async fn register(req: &mut Request, res: &mut Response, depot: &mut Depot)->Result<(),UniformError>{
    let base_url = depot.get::<String>("base_url").to_result()?;
    let tera = depot.get::<Tera>("tera").to_result()?;
    if req.method() == salvo::http::Method::GET {
        let mut context = Context::new();
        context.insert("baseUrl",base_url);
        let r = tera.render("reg.html", &context)?;
        res.render(Text::Html(r));
    }else{
        let db = depot.get::<DatabaseConnection>("db_conn").to_result()?;
        let count = UserTb::find().filter(user_tb::Column::Name.eq("test2")).count(db).await?;
        res.render(Text::Plain(count.to_string()));
    }
    Ok(())
}

#[handler]
pub async fn author(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    if let Some(session) = depot.session_mut() {
        let name = session.get::<String>("name");
        //println!("{name:?}");
        let val = format!("{:?}", name);

        res.render(Text::Html(val));
    } else {
        res.with_status_code(StatusCode::BAD_GATEWAY)
            .render("invalid user");
    }
}

#[handler]
pub async fn logout(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    if let Some(session) = depot.session_mut() {
        session.remove("name");
        res.render("Success");
    } else {
        res.with_status_code(StatusCode::BAD_GATEWAY)
            .render("session error");
    }
}
