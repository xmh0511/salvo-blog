mod database;

use database::prelude::*;

use redis::TypedCommands;
use resend_rs::{Resend, types::CreateEmailBaseOptions};
use salvo::prelude::*;
use sea_orm::{DatabaseBackend, DatabaseConnection, EntityTrait, JsonValue, Statement, prelude::*};

use serde_json::json;

use url_encor::Encoder;

use self::database::{article_tb, comment_tb, tag_tb, user_tb, view_tb};

use sea_orm::{entity::*, query::*};

use salvo::http::StatusCode;

use tera::{Context, Tera};
use time::{Duration, OffsetDateTime};

use chrono::prelude::*;
use jsonwebtoken::{self, EncodingKey};

use ::serde::{Deserialize, Serialize};

macro_rules! construct_context {
    ($($k:expr => $v:expr),+) => {
        {
            let mut context = Context::new();
            $(context.insert($k,&$v);)+
            context
        }
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub user_id: String,
    pub exp: i64,
}

const RESPONSE_TEXT_FOR_ERROR: u8 = 1;
const RESPONSE_JSON_FOR_ERROR: u8 = 2;

#[derive(Debug)]
pub struct UniformError<const ERRORCODE: u8 = 1>(anyhow::Error);

impl<const ERRORCODE: u8, T: Into<anyhow::Error>> From<T> for UniformError<ERRORCODE> {
    fn from(v: T) -> Self {
        UniformError(v.into())
    }
}

#[async_trait]
impl<const ERRORCODE: u8> Writer for UniformError<ERRORCODE> {
    async fn write(mut self, _req: &mut Request, depot: &mut Depot, res: &mut Response) {
        let err = self.0.to_string();
        if ERRORCODE == 1 {
            let Ok(tera) = depot.get::<Tera>("tera") else {
                res.status_code(StatusCode::BAD_REQUEST)
                    .render(Text::Plain(err));
                return;
            };
            let default_url = "/".to_string();
            let base_url = depot.get::<String>("base_url").unwrap_or(&default_url);
            let context = construct_context!["code"=>404,"msg"=>err,"baseUrl"=>base_url];
            let r = tera.render("404.html", &context).unwrap_or(err);

            res.status_code(StatusCode::BAD_REQUEST)
                .render(Text::Html(r));
        } else {
            let r = json!({
                "code":400,
                "msg":self.0.to_string()
            });
            res.status_code(StatusCode::BAD_REQUEST)
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
async fn get_relative_information_from_article(
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

async fn get_hot_article_list(db: &DatabaseConnection) -> Result<Vec<JsonValue>, UniformError> {
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
                WHERE
                    a.`level` <> 999 
                    AND a.article_state = 1 
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

async fn get_person_right_state<const I: u8>(
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

async fn generate_token_by_user_id<const I: u8>(
    secret_key: &str,
    user_id: i32,
    remember: bool,
) -> Result<String, UniformError<I>> {
    let exp = OffsetDateTime::now_utc() + Duration::days(if remember { 30 } else { 1 });
    let claim = JwtClaims {
        user_id: user_id.to_string(),
        exp: exp.unix_timestamp(),
    };
    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claim,
        &EncodingKey::from_secret(secret_key.as_bytes()),
    )?;
    Ok(token)
}

#[handler]
pub async fn home(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let page = match req.param::<u64>("page") {
        Some(x) if x >= 1 => x - 1,
        _ => {
            let uri = format!("{base_url}home/1");
            res.render(Redirect::other(uri));
            return Ok(());
        }
    };
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let pagination = ArticleTb::find()
        .order_by_desc(article_tb::Column::UpdateTime)
        .filter(article_tb::Column::ArticleState.eq(1))
        .filter(article_tb::Column::Level.ne(999))
        .into_json()
        .paginate(db, 10);
    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    let total_count = pagination.num_items().await?;
    let total_pages = pagination.num_pages().await?;
    if page != 0 && (page + 1) > total_pages {
        return Err(UniformError(anyhow::anyhow!("请求的资源不存在")));
    }
    let mut data = pagination.fetch_page(page).await?;

    for model in &mut data {
        let id = model.get("id").to_result()?.as_u64().to_result()?;
        let tag_id = model.get("tag_id").to_result()?.as_u64().to_result()?;
        let user_id = model.get("user_id").to_result()?.as_u64().to_result()?;
        let result = get_relative_information_from_article(id, user_id, tag_id, db).await?;
        model["userName"] = json!(result.0.name);
        model["tagName"] = json!(result.1.name);
        model["read_count"] = json!(result.2);
        model["commentCount"] = json!(result.3);
    }

    let hot_list = get_hot_article_list(db).await?;
    let mut context = Context::new();
    let login_data = 'login_data: {
        match depot.jwt_auth_state() {
            JwtAuthState::Authorized => {
                let data = depot.jwt_auth_data::<JwtClaims>().to_result()?;
                let Ok(info) = get_person_right_state::<RESPONSE_TEXT_FOR_ERROR>(
                    data.claims.user_id.as_str().parse()?,
                    db,
                )
                .await
                else {
                    break 'login_data json!({
                        "login":false,
                        "avatar":""
                    });
                };
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
    context.insert("total", &total_count);
    context.insert("commentCount", &10);
    context.insert("page", &(page + 1));
    context.insert("hotArticles", &hot_list);
    let r = tera.render("home.html", &context)?;
    res.render(Text::Html(r));
    Ok(())
}

#[handler]
pub async fn render_login_view(
    _req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    let context = construct_context!["baseUrl"=>base_url];
    let r = tera.render("login.html", &context)?;
    res.render(Text::Html(r));
    Ok(())
}

#[handler]
pub async fn login(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let name = req.form::<String>("nickName").await.to_result()?;
    let pass = req.form::<String>("password").await.to_result()?;
    let remember_me = req.form::<String>("rememberMe").await.to_result()?;
    let pass = md5::compute(pass);
    let pass = format!("{:?}", pass);
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let Some(r) = UserTb::find()
        .filter(user_tb::Column::Name.eq(name.clone()))
        .filter(user_tb::Column::Password.eq(pass))
        .one(db)
        .await?
    else {
        let r = json!({
           "code":400,
           "msg": "用户名或密码错误",
           "baseUrl":base_url
        });
        res.render(Text::Json(r.to_string()));
        return Ok(());
    };
    let remember = remember_me.trim() == "true";
    let secret_key = depot
        .get::<String>("secret_key")
        .map_err(|_| anyhow::anyhow!("failed to acquire signing key"))?;
    let token = generate_token_by_user_id(secret_key, r.id, remember).await?;
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
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let page = match req.param::<u64>("page") {
        Some(x) if x >= 1 => x - 1,
        _ => {
            let uri = format!("{base_url}list/1");
            res.render(Redirect::other(uri));
            return Ok(());
        }
    };
    let data = depot.jwt_auth_data::<JwtClaims>().to_result()?;
    let user_id = data.claims.user_id.clone();
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let offset = page * 10;
    let sql = r#"SELECT
	R.AID,
	R.`name`,
	R.title,
	R.view_count,
	R.update_time,
	R.article_state,
    R.level,
	COUNT( comment_tb.id ) AS comment_count 
FROM
	(
	SELECT
		tag_tb.`name`,
		article_tb.title,
		article_tb.id AS AID,
		article_tb.update_time,
		article_tb.article_state,
        article_tb.level,
		COUNT( view_tb.article_id ) AS view_count 
	FROM
		article_tb
		LEFT JOIN tag_tb ON tag_tb.id = article_tb.tag_id
		LEFT JOIN view_tb ON view_tb.article_id = article_tb.id 
	WHERE
		article_tb.user_id = ?
	GROUP BY
		AID 
	) AS R
	LEFT JOIN comment_tb ON comment_tb.article_id = R.AID 
GROUP BY
	AID 
ORDER BY
	R.update_time DESC
	LIMIT ?, 10"#;
    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    let total_count = ArticleTb::find()
        .filter(article_tb::Column::UserId.eq(user_id.as_str()))
        .count(db)
        .await?;
    let total_page = if total_count >= 10 {
        let modulo = total_count % 10;
        if modulo > 0 {
            total_count / 10 + 1
        } else {
            total_count / 10
        }
    } else {
        1
    };
    if page != 0 && page + 1 > total_page {
        return Err(UniformError(anyhow::anyhow!("请求的资源不存在")));
    }
    let sql_statement = Statement::from_sql_and_values(
        DatabaseBackend::MySql,
        sql,
        [
            Value::BigInt(Some(user_id.parse()?)),
            Value::BigUnsigned(Some(offset)),
        ],
    );
    let r = ArticleTb::find()
        .from_raw_sql(sql_statement)
        .into_json()
        .all(db)
        .await?;
    let info = get_person_right_state(user_id.parse()?, db).await?;
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

    let mut context = Context::new();
    let hot_list = get_hot_article_list(db).await?;
    context.insert("articles", &r);
    context.insert("login", &login_v);
    context.insert("page", &(page + 1));
    context.insert("total", &total_count);
    context.insert("baseUrl", &base_url);
    context.insert("hotArticles", &hot_list);
    let r = tera.render("list.html", &context)?;
    res.render(Text::Html(r));
    Ok(())
}

#[handler]
pub async fn register(
    _req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    let mut context = Context::new();
    context.insert("baseUrl", base_url);
    let r = tera.render("reg.html", &context)?;
    res.render(Text::Html(r));
    Ok(())
}

#[handler]
pub async fn post_register(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let name = req.form::<String>("nickName").await.to_result()?;
    let pass = req.form::<String>("password").await.to_result()?;
    let confirm_pass = req.form::<String>("password2").await.to_result()?;
    let email = req
        .form::<String>("email")
        .await
        .ok_or(anyhow::anyhow!("email is required"))?;
    let email_code = req
        .form::<String>("code")
        .await
        .ok_or(anyhow::anyhow!("code is required"))?;

    let redis_url = depot
        .get::<String>("redis_url")
        .map_err(|_| anyhow::anyhow!("email system is not ready for db"))?;
    let client = redis::Client::open(redis_url.as_str())?;
    let mut con = client.get_connection()?;
    let code = con
        .get(&email)?
        .ok_or(anyhow::anyhow!("未通过邮箱验证, 未找到邮箱的验证码"))?;
    if code != email_code {
        let r = json!({
            "code":400,
            "msg":"邮箱验证码错误"
        });
        res.render(Text::Json(r.to_string()));
        return Ok(());
    }

    if pass.chars().count() < 6 {
        let r = json!({
            "code":400,
            "msg":"密码长度少于6位"
        });
        res.render(Text::Json(r.to_string()));
        return Ok(());
    }
    if pass != confirm_pass {
        let r = json!({
            "code":400,
            "msg":"密码不一致"
        });
        res.render(Text::Json(r.to_string()));
        return Ok(());
    }

    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let count = UserTb::find()
        .filter(user_tb::Column::Name.eq(name.clone()))
        .count(db)
        .await?;
    if count != 0 {
        let r = json!({
            "code":400,
            "msg":"用户名已存在"
        });
        res.render(Text::Json(r.to_string()));
        return Ok(());
    } else {
        let mut add_user = user_tb::ActiveModel::new();
        add_user.avatar = ActiveValue::set(None);
        let time_now = Local::now();
        add_user.create_time = ActiveValue::set(Some(time_now.naive_local()));
        add_user.email = ActiveValue::set(Some(email));
        add_user.name = ActiveValue::set(Some(name));
        let pass = format!("{:?}", md5::compute(pass));
        add_user.password = ActiveValue::set(Some(pass));
        add_user.update_time = ActiveValue::set(Some(time_now.naive_local()));
        add_user.privilege = ActiveValue::set(Some(2));
        let r = UserTb::insert(add_user).exec(db).await?.last_insert_id;
        let secret_key = depot
            .get::<String>("secret_key")
            .map_err(|_| anyhow::anyhow!("failed to acquire signing key"))?;
        let token = generate_token_by_user_id(secret_key, r, false).await?;
        let base_url = depot
            .get::<String>("base_url")
            .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
        let r = json!({
           "code":200,
           "token":token,
           "msg":"注册成功",
           "baseUrl":base_url
        });
        res.render(Text::Json(r.to_string()));
    }
    Ok(())
}

#[handler]
pub async fn forgetpass(
    _req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    let mut context = Context::new();
    context.insert("baseUrl", base_url);
    let r = tera.render("forget.html", &context)?;
    res.render(Text::Html(r));
    Ok(())
}

#[handler]
pub async fn post_forget(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let name = req.form::<String>("nickName").await.to_result()?;
    let pass = req.form::<String>("password").await.to_result()?;
    let confirm_pass = req.form::<String>("password2").await.to_result()?;
    let email = req
        .form::<String>("email")
        .await
        .ok_or(anyhow::anyhow!("email is required"))?;
    let email_code = req
        .form::<String>("code")
        .await
        .ok_or(anyhow::anyhow!("code is required"))?;

    let redis_url = depot
        .get::<String>("redis_url")
        .map_err(|_| anyhow::anyhow!("email system is not ready for db"))?;
    let client = redis::Client::open(redis_url.as_str())?;
    let mut con = client.get_connection()?;
    let code = con
        .get(&email)?
        .ok_or(anyhow::anyhow!("未通过邮箱验证, 未找到邮箱的验证码"))?;
    if code != email_code {
        let r = json!({
            "code":400,
            "msg":"邮箱验证码错误"
        });
        res.render(Text::Json(r.to_string()));
        return Ok(());
    }

    if pass.chars().count() < 6 {
        let r = json!({
            "code":400,
            "msg":"密码长度少于6位"
        });
        res.render(Text::Json(r.to_string()));
        return Ok(());
    }
    if pass != confirm_pass {
        let r = json!({
            "code":400,
            "msg":"密码不一致"
        });
        res.render(Text::Json(r.to_string()));
        return Ok(());
    }

    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let user = UserTb::find()
        .filter(user_tb::Column::Name.eq(name.clone()))
        .filter(user_tb::Column::Email.eq(&email))
        .one(db)
        .await?;
    if let Some(user) = user {
        let mut user = user_tb::ActiveModel::from(user);
        let pass = format!("{:?}", md5::compute(pass));
        user.password = ActiveValue::set(Some(pass));
        let time_now = Local::now();
        user.update_time = ActiveValue::set(Some(time_now.naive_local()));
        user.update(db).await?;
        let base_url = depot
            .get::<String>("base_url")
            .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
        let r = json!({
           "code":200,
           "msg":"重置账号成功",
           "baseUrl":base_url
        });
        res.render(Text::Json(r.to_string()));
    } else {
        let r = json!({
            "code":400,
            "msg":"重置的用户不存在或与邮箱不匹配"
        });
        res.render(Text::Json(r.to_string()));
    }
    Ok(())
}

async fn get_comments_from_article_id(
    article_id: i32,
    db: &DatabaseConnection,
) -> Result<Vec<JsonValue>, UniformError> {
    let sql = r#"SELECT
	comment_tb.id,
	comment_tb.`comment`,
	comment_tb.md_content,
    comment_tb.create_time,
	comment_tb.update_time,
	user_tb.id AS user_id,
	user_tb.avatar,
	user_tb.`name` AS userName,
	user_tb.privilege AS level 
FROM
	comment_tb
	LEFT JOIN user_tb ON comment_tb.user_id = user_tb.id 
WHERE
	comment_tb.article_id = ?
ORDER BY
	comment_tb.create_time"#;
    let sql_statement =
        Statement::from_sql_and_values(DatabaseBackend::MySql, sql, [Value::Int(Some(article_id))]);
    let r = CommentTb::find()
        .from_raw_sql(sql_statement)
        .into_json()
        .all(db)
        .await?;
    Ok(r)
}

async fn get_article_and_author_by_article_id(
    article_id: i32,
    db: &DatabaseConnection,
) -> Result<JsonValue, UniformError> {
    let sql = r#"SELECT
	article_tb.id,
	article_tb.title,
	article_tb.create_time,
	article_tb.update_time,
	article_tb.`level`,
	article_tb.content,
	user_tb.`name` as userName,
    article_tb.user_id as user_id
FROM
	article_tb
	LEFT JOIN user_tb ON user_tb.id = article_tb.user_id 
WHERE
	article_tb.id = ?"#;
    let sql_statement =
        Statement::from_sql_and_values(DatabaseBackend::MySql, sql, [Value::Int(Some(article_id))]);
    let r = ArticleTb::find()
        .from_raw_sql(sql_statement)
        .into_json()
        .one(db)
        .await?
        .to_result()?;
    Ok(r)
}

async fn increase_view_count(article_id: i32, db: &DatabaseConnection) -> Result<(), UniformError> {
    let mut model = view_tb::ActiveModel::new();
    model.article_id = ActiveValue::set(Some(article_id));
    let now = ActiveValue::set(Some(Local::now().naive_local()));
    model.create_time = now.clone();
    model.insert(db).await?;
    Ok(())
}

#[handler]
pub async fn read_article(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let article_id: i32 = req.param("id").to_result()?;

    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;

    let article_model = get_article_and_author_by_article_id(article_id, db).await?;

    let need_level = article_model
        .get("level")
        .to_result()?
        .as_u64()
        .to_result()?;

    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;

    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    match depot.jwt_auth_state() {
        JwtAuthState::Authorized => {
            let data = depot.jwt_auth_data::<JwtClaims>().to_result()?;
            let user_id = &data.claims.user_id;
            let person = UserTb::find_by_id(user_id.parse::<i32>()?)
                .one(db)
                .await?
                .to_result()?;
            let currend_id = data.claims.user_id.parse::<u64>()?;
            if need_level == 999 {
                if currend_id
                    == article_model
                        .get("user_id")
                        .to_result()?
                        .as_u64()
                        .to_result()?
                {
                    let comments = get_comments_from_article_id(article_id, db).await?;
                    let context = construct_context!["info"=>article_model,"comments"=>comments,"baseUrl"=>base_url,"currentId"=>currend_id];
                    let r = tera.render("article.html", &context)?;
                    res.render(Text::Html(r));
                } else {
                    let context = construct_context!["code"=>404, "msg"=>"没有该文章的阅读权限","baseUrl"=>base_url];
                    let r = tera.render("404.html", &context)?;
                    res.render(Text::Html(r));
                }
            } else if need_level <= person.privilege.unwrap_or(1) as u64 {
                increase_view_count(article_id, db).await?;
                let comments = get_comments_from_article_id(article_id, db).await?;
                let context = construct_context!["info"=>article_model,"comments"=>comments,"baseUrl"=>base_url,"currentId"=>currend_id];
                let r = tera.render("article.html", &context)?;
                res.render(Text::Html(r));
            } else {
                let context = construct_context!["code"=>404, "msg"=>"没有该文章的阅读权限","baseUrl"=>base_url];
                let r = tera.render("404.html", &context)?;
                res.render(Text::Html(r));
            }
        }
        JwtAuthState::Unauthorized => {
            if need_level == 999 {
                let context = construct_context!["code"=>404, "msg"=>"没有该文章的阅读权限","baseUrl"=>base_url];
                let r = tera.render("404.html", &context)?;
                res.render(Text::Html(r));
            } else if need_level <= 1 {
                increase_view_count(article_id, db).await?;
                let comments = get_comments_from_article_id(article_id, db).await?;
                let context = construct_context!["info"=>article_model,"comments"=>comments,"baseUrl"=>base_url,"currentId"=>Option::<i32>::None];
                let r = tera.render("article.html", &context)?;
                res.render(Text::Html(r));
            } else {
                let context = construct_context!["code"=>404, "msg"=>"没有该文章的阅读权限","baseUrl"=>base_url];
                let r = tera.render("404.html", &context)?;
                res.render(Text::Html(r));
            }
        }
        JwtAuthState::Forbidden => {
            if need_level == 999 {
                let context = construct_context!["code"=>404, "msg"=>"没有该文章的阅读权限","baseUrl"=>base_url];
                let r = tera.render("404.html", &context)?;
                res.render(Text::Html(r));
            } else if need_level <= 1 {
                increase_view_count(article_id, db).await?;
                let comments = get_comments_from_article_id(article_id, db).await?;
                let context = construct_context!["info"=>article_model,"comments"=>comments,"baseUrl"=>base_url,"currentId"=>Option::<i32>::None];
                let r = tera.render("article.html", &context)?;
                res.render(Text::Html(r));
            } else {
                let context = construct_context!["code"=>404, "msg"=>"没有该文章的阅读权限","baseUrl"=>base_url];
                let r = tera.render("404.html", &context)?;
                res.render(Text::Html(r));
            }
        }
    };
    Ok(())
}

#[handler]
pub async fn delete_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let comment_id = req.param::<i32>("id").to_result()?;
    let identifier = &depot
        .jwt_auth_data::<JwtClaims>()
        .to_result()?
        .claims
        .user_id;
    let identifier = identifier.as_str();
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let r = CommentTb::find_by_id(comment_id)
        .filter(comment_tb::Column::UserId.eq(identifier))
        .count(db)
        .await?;
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    if r == 1 {
        let _ = CommentTb::delete_by_id(comment_id).exec(db).await?;
        let r = json!({
            "code":200,
            "baseUrl":base_url
        });
        res.render(Text::Json(r.to_string()));
    } else {
        let r = json!({
            "code":404,
            "msg":"没有权限执行此操作",
            "baseUrl":base_url
        });
        res.render(Text::Json(r.to_string()));
    }
    Ok(())
}

#[handler]
pub async fn edit_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let comment_id = req.param::<i32>("id").to_result()?;
    let identifier = &depot
        .jwt_auth_data::<JwtClaims>()
        .to_result()?
        .claims
        .user_id;
    let identifier = identifier.as_str();
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let r = CommentTb::find_by_id(comment_id)
        .filter(comment_tb::Column::UserId.eq(identifier))
        .into_json()
        .one(db)
        .await?;
    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    if let Some(x) = r {
        let context = construct_context!["info"=>x,"baseUrl"=>base_url];
        let r = tera.render("editcomment.html", &context)?;
        res.render(Text::Html(r));
    } else {
        let context =
            construct_context!["code"=>404,"msg"=>"没有权限执行此操作","baseUrl"=>base_url];
        let r = tera.render("404.html", &context)?;
        res.render(Text::Html(r));
    }
    Ok(())
}

#[handler]
pub async fn save_edit_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let comment_id = req.param::<i32>("id").to_result()?;
    let comment: String = req.form("comment").await.to_result()?;
    let md_content: String = req.form("md_content").await.to_result()?;
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let identifier = &depot
        .jwt_auth_data::<JwtClaims>()
        .to_result()?
        .claims
        .user_id;
    let identifier = identifier.as_str();
    //let tera = depot.get::<Tera>("tera").to_result()?;
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let model = CommentTb::find_by_id(comment_id)
        .filter(comment_tb::Column::UserId.eq(identifier))
        .one(db)
        .await?;
    if let Some(x) = model {
        let mut update = comment_tb::ActiveModel::from(x);
        update.comment = ActiveValue::set(Some(comment));
        update.md_content = ActiveValue::set(Some(md_content));
        update.update_time = ActiveValue::set(Some(Local::now().naive_local()));
        let _ = update.update(db).await?;
        let r = json!({
            "code":200,
        });
        res.render(Text::Json(r.to_string()));
    } else {
        let r = json!({
            "code":404,
            "msg":"没有权限执行此操作",
            "baseUrl":base_url
        });
        res.render(Text::Json(r.to_string()));
    }
    Ok(())
}

#[handler]
pub async fn add_comment(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let identifier = &depot
        .jwt_auth_data::<JwtClaims>()
        .to_result()?
        .claims
        .user_id;
    let identifier = identifier.parse()?;
    let article_id = req.param::<i32>("id").to_result()?;
    let comment: String = req.form("comment").await.to_result()?;
    let md_comment: String = req.form("md_content").await.to_result()?;
    let mut model = comment_tb::ActiveModel::new();
    model.article_id = ActiveValue::set(Some(article_id));
    model.comment = ActiveValue::set(Some(comment));
    let now = ActiveValue::set(Some(Local::now().naive_local()));
    model.create_time = now.clone();
    model.md_content = ActiveValue::set(Some(md_comment));
    model.update_time = now;
    model.user_id = ActiveValue::set(Some(identifier));
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let _ = model.insert(db).await?;
    let r = json!({
        "code":200
    });
    res.render(Text::Json(r.to_string()));
    Ok(())
}

#[handler]
pub async fn upload(
    req: &mut Request,
    res: &mut Response,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let file = req.file("editormd-image-file").await;
    if let Some(file) = file {
        let path = file.path();
        let file_name = path.file_name().to_result()?.to_str().to_result()?;
        let dest = format!("./public/upload/{}", file_name);
        let url = format!("upload/{}", file_name);
        let _ = std::fs::copy(path, dest.clone())?;
        let r = json!({
            "success":1,
            "message":"",
            "url":url
        });
        res.render(Text::Json(r.to_string()));
    } else {
        res.status_code(StatusCode::BAD_REQUEST);
        let r = json!({
            "success":0,
            "message":"file not found in request",
        });
        res.render(Text::Json(r.to_string()));
    };
    Ok(())
}

#[handler]
pub async fn render_add_article_view(
    _req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let tags = TagTb::find().into_json().all(db).await?;
    let levels = LevelTb::find().into_json().all(db).await?;
    let context = construct_context!["tags"=>tags,"levels"=>levels,"baseUrl"=>base_url];
    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    let r = tera.render("add.html", &context)?;
    res.render(Text::Html(r));
    Ok(())
}

#[handler]
pub async fn add_article(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let tag = req.form::<i32>("tag").await.to_result()?;
    let title = req.form::<String>("title").await.to_result()?;
    let content = req.form::<String>("content").await.to_result()?;
    let level = req.form::<i32>("level").await.to_result()?;
    if title.is_empty() || content.is_empty() {
        let r = json!({
            "code":404,
            "msg":"填写完整信息"
        });
        res.render(Text::Json(r.to_string()));
    } else {
        let identifier = &depot
            .jwt_auth_data::<JwtClaims>()
            .to_result()?
            .claims
            .user_id;
        let identifier = identifier.as_str();
        let base_url = depot
            .get::<String>("base_url")
            .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
        let db = depot
            .get::<DatabaseConnection>("db_conn")
            .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
        let mut model = article_tb::ActiveModel::new();
        model.article_state = ActiveValue::set(Some(1));
        model.content = ActiveValue::set(Some(content));
        let now = ActiveValue::set(Some(Local::now().naive_local()));
        model.create_time = now.clone();
        model.level = ActiveValue::set(Some(level));
        model.tag_id = ActiveValue::set(Some(tag));
        model.title = ActiveValue::set(Some(title));
        model.update_time = now;
        model.user_id = ActiveValue::set(Some(identifier.parse()?));
        model.insert(db).await?;
        let r = json!({
            "code":200,
            "baseUrl":base_url
        });
        res.render(Text::Json(r.to_string()));
    }
    Ok(())
}

#[handler]
pub async fn render_article_edit_view(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let article_id = req.param::<i32>("id").to_result()?;

    let identifier = &depot
        .jwt_auth_data::<JwtClaims>()
        .to_result()?
        .claims
        .user_id;

    let identifier = identifier.as_str();

    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;

    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;

    let model = ArticleTb::find_by_id(article_id)
        .filter(article_tb::Column::UserId.eq(identifier))
        .into_json()
        .one(db)
        .await?
        .to_result()?;

    let tags = TagTb::find().into_json().all(db).await?;
    let levels = LevelTb::find().into_json().all(db).await?;
    let context =
        construct_context!["tags"=>tags,"levels"=>levels,"baseUrl"=>base_url,"article"=>model];
    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    let r = tera.render("edit.html", &context)?;
    res.render(Text::Html(r));
    Ok(())
}

#[handler]
pub async fn edit_article(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let article_id = req.param::<i32>("id").to_result()?;

    let identifier = &depot
        .jwt_auth_data::<JwtClaims>()
        .to_result()?
        .claims
        .user_id;
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;

    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let model = ArticleTb::find_by_id(article_id)
        .filter(article_tb::Column::UserId.eq(identifier.as_str()))
        .one(db)
        .await?
        .to_result()?;
    let mut model = article_tb::ActiveModel::from(model);
    let tag = req.form::<i32>("tag").await.to_result()?;
    let title = req.form::<String>("title").await.to_result()?;
    let content = req.form::<String>("content").await.to_result()?;
    let level = req.form::<i32>("level").await.to_result()?;
    model.tag_id = ActiveValue::set(Some(tag));
    model.title = ActiveValue::set(Some(title));
    model.content = ActiveValue::set(Some(content));
    model.level = ActiveValue::set(Some(level));
    model.update_time = ActiveValue::set(Some(Local::now().naive_local()));
    model.update(db).await?;
    let r = json!({
        "code":200,
        "baseUrl":base_url
    });
    res.render(Text::Json(r.to_string()));
    Ok(())
}

#[handler]
pub async fn shadow_article(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let article_id = req.param::<i32>("id").to_result()?;

    let identifier = &depot
        .jwt_auth_data::<JwtClaims>()
        .to_result()?
        .claims
        .user_id;
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let model = ArticleTb::find_by_id(article_id)
        .filter(article_tb::Column::UserId.eq(identifier.as_str()))
        .one(db)
        .await?
        .to_result()?;
    let state = model.article_state.unwrap_or(1);
    let mut model = article_tb::ActiveModel::from(model);
    model.article_state = {
        if state == 1 {
            ActiveValue::set(Some(0))
        } else {
            ActiveValue::set(Some(1))
        }
    };
    model.update_time = ActiveValue::set(Some(Local::now().naive_local()));
    model.update(db).await?;
    let r = json!({
        "code":200,
        "baseUrl":base_url
    });
    res.render(Text::Json(r.to_string()));
    Ok(())
}

#[handler]
pub async fn render_profile_view(
    _req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let identifier = &depot
        .jwt_auth_data::<JwtClaims>()
        .to_result()?
        .claims
        .user_id;
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let model = UserTb::find_by_id(identifier.parse::<i32>()?)
        .into_json()
        .one(db)
        .await?
        .to_result()?;
    let context = construct_context!["info"=>model,"baseUrl"=>base_url];
    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    let r = tera.render("person.html", &context)?;
    res.render(Text::Html(r));
    Ok(())
}

#[handler]
pub async fn edit_profile(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError<RESPONSE_JSON_FOR_ERROR>> {
    let identifier = &depot
        .jwt_auth_data::<JwtClaims>()
        .to_result()?
        .claims
        .user_id;
    let avatar = req.form::<String>("path").await.to_result()?;
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    if avatar.is_empty() {
        let r = json!({
            "code":404,
            "msg":"填写完整信息",
            "baseUrl":base_url
        });
        res.render(Text::Json(r.to_string()));
    } else {
        let db = depot
            .get::<DatabaseConnection>("db_conn")
            .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
        let model = UserTb::find_by_id(identifier.parse::<i32>()?)
            .one(db)
            .await?
            .to_result()?;
        let mut model = user_tb::ActiveModel::from(model);
        model.avatar = ActiveValue::set(Some(avatar));
        model.update_time = ActiveValue::set(Some(Local::now().naive_local()));
        model.update(db).await?;
        let r = json!({
            "code":200,
            "baseUrl":base_url
        });
        res.render(Text::Json(r.to_string()));
    }
    Ok(())
}

#[handler]
pub async fn search(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let base_url = depot
        .get::<String>("base_url")
        .map_err(|_| anyhow::anyhow!("failed to acquire base url"))?;
    let query_key = req.query("query").unwrap_or("");
    //println!("raw query_key = {}",query_key);
    let page = match req.param::<u64>("page") {
        Some(x) if x >= 1 => x - 1,
        _ => {
            let query_key = query_key.url_encode();
            //println!("encode query_key = {}",query_key);
            let uri = format!("{base_url}search/1/?query={query_key}");
            res.render(Redirect::other(uri));
            return Ok(());
        }
    };
    // let query_key = query_key.url_decode();
    //println!("query_key = {}",query_key);
    let db = depot
        .get::<DatabaseConnection>("db_conn")
        .map_err(|_| anyhow::anyhow!("failed to acquire db connection"))?;
    let query_condition = format!("%{query_key}%");
    let possible_tags = TagTb::find()
        .filter(tag_tb::Column::Name.like(&query_condition))
        .all(db)
        .await?;
    let mut filter_condition = Condition::any().add(
        article_tb::Column::Title
            .like(&query_condition)
            .add(article_tb::Column::Content.like(&query_condition)),
    );
    for tag in possible_tags {
        filter_condition = filter_condition.add(article_tb::Column::TagId.eq(tag.id));
    }
    let pagination = ArticleTb::find()
        .order_by_desc(article_tb::Column::UpdateTime)
        .filter(article_tb::Column::ArticleState.eq(1))
        .filter(article_tb::Column::Level.ne(999))
        .filter(filter_condition)
        .into_json()
        .paginate(db, 10);

    let tera = depot
        .get::<Tera>("tera")
        .map_err(|_| anyhow::anyhow!("failed to acquire tera engine"))?;
    let total_count = pagination.num_items().await?;
    let total_pages = pagination.num_pages().await?;
    if page != 0 && (page + 1) > total_pages {
        return Err(UniformError(anyhow::anyhow!("请求的资源不存在")));
    }
    let mut data = pagination.fetch_page(page).await?;

    for model in &mut data {
        let id = model.get("id").to_result()?.as_u64().to_result()?;
        let tag_id = model.get("tag_id").to_result()?.as_u64().to_result()?;
        let user_id = model.get("user_id").to_result()?.as_u64().to_result()?;
        let result = get_relative_information_from_article(id, user_id, tag_id, db).await?;
        model["userName"] = json!(result.0.name);
        model["tagName"] = json!(result.1.name);
        model["read_count"] = json!(result.2);
        model["commentCount"] = json!(result.3);
    }

    let hot_list = get_hot_article_list(db).await?;
    let mut context = Context::new();
    let login_data = 'login_data: {
        match depot.jwt_auth_state() {
            JwtAuthState::Authorized => {
                let data = depot.jwt_auth_data::<JwtClaims>().to_result()?;
                let Ok(info) = get_person_right_state::<RESPONSE_TEXT_FOR_ERROR>(
                    data.claims.user_id.as_str().parse()?,
                    db,
                )
                .await
                else {
                    break 'login_data json!({
                        "login":false,
                        "avatar":""
                    });
                };
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
                json!({
                    "login":false,
                    "avatar":""
                })
            }
        }
    };
    let encoded_query_key = query_key.url_encode();
    context.insert("baseUrl", base_url);
    context.insert("login", &login_data);
    context.insert("articles", &data);
    context.insert("total", &total_count);
    context.insert("raw_query", &query_key);
    context.insert("query", &encoded_query_key);
    context.insert("commentCount", &10);
    context.insert("page", &(page + 1));
    context.insert("hotArticles", &hot_list);
    let r = tera.render("search.html", &context)?;
    res.render(Text::Html(r));
    Ok(())
}

fn gen_code() -> String {
    [
        rand::random_range(0..9).to_string(),
        rand::random_range(0..9).to_string(),
        rand::random_range(0..9).to_string(),
        rand::random_range(0..9).to_string(),
    ]
    .join("")
}

#[handler]
pub async fn sendcode(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), UniformError> {
    let email = req
        .form::<String>("email")
        .await
        .ok_or(anyhow::anyhow!("email is required"))?;
    if !checkmail::validate_email(&email) {
        let r = json!({
            "code":400,
            "msg":"无效的邮箱"
        });
        res.render(Text::Json(r.to_string()));
        return Ok(());
    }
    let resend_key = depot
        .get::<String>("resend_key")
        .map_err(|_| anyhow::anyhow!("email system is not ready for resend"))?;
    let redis_url = depot
        .get::<String>("redis_url")
        .map_err(|_| anyhow::anyhow!("email system is not ready for db"))?;
    let client = redis::Client::open(redis_url.as_str())?;
    let mut con = client.get_connection()?;
    let code = gen_code();
    con.set_ex(&email, &code, 5 * 60)?;

    let resend = Resend::new(resend_key);
    let from = "Blog <blog@resend.dev>";
    let to = [email];
    let subject = "你好，邮箱验证码";
    let html = format!("<p>验证码5分钟内有效, 你的验证码：{}</p>", code);
    let email = CreateEmailBaseOptions::new(from, to, subject).with_html(&html);
    let _email = resend.emails.send(email).await?;

    let r = json!({
        "code":200,
    });
    res.render(Text::Json(r.to_string()));
    Ok(())
}

// #[handler]
// pub async fn getcode(
//     req: &mut Request,
//     res: &mut Response,
//     depot: &mut Depot,
// ) -> Result<(), UniformError> {
//     let email = req
//         .form::<String>("email")
//         .await
//         .ok_or(anyhow::anyhow!("email is required"))?;
//     let redis_url = depot
//         .get::<String>("redis_url")
//         .map_err(|_| anyhow::anyhow!("email system is not ready for db"))?;
//     let client = redis::Client::open(redis_url.as_str())?;
//     let mut con = client.get_connection()?;
//     let code = con.get(&email)?;
//     let r = json!({
//         "code":200,
//         "msg":&code
//     });
//     res.render(Text::Json(r.to_string()));
//     Ok(())
// }
