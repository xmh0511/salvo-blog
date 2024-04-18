use salvo::{catcher::Catcher, prelude::*};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
mod home;

use salvo::serve_static::StaticDir;

use serde_json::json;
use tokio::fs;

use tera::{Context, Tera};

use salvo::jwt_auth::{CookieFinder,ConstDecoder};

use home::{JwtClaims,UniformError};
use tracing::log;

macro_rules! share_db {
    ($id:ident) => {
        InjectDbConnection($id.clone())
    };
}

#[derive(Clone)]
struct InjectDbConnection(DatabaseConnection);

#[async_trait]
impl Handler for InjectDbConnection {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        depot.insert("db_conn", self.0.clone());
        ctrl.call_next(req, depot, res).await;
    }
}

#[derive(Clone)]
struct BaseUrlInjector(String);

#[async_trait]
impl Handler for BaseUrlInjector {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        depot.insert("base_url", self.0.clone());
        ctrl.call_next(req, depot, res).await;
    }
}
#[derive(Clone)]
struct TemplateEngineInjection(Tera);
#[async_trait]
impl Handler for TemplateEngineInjection {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        depot.insert("tera", self.0.clone());
        ctrl.call_next(req, depot, res).await;
    }
}

#[derive(Clone)]
struct AuthorGuardByMethod;

#[handler]
impl AuthorGuardByMethod {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) -> Result<(), UniformError> {
        match depot.jwt_auth_state() {
            JwtAuthState::Authorized => {
                ctrl.call_next(req, depot, res).await;
                Ok(())
            }
            JwtAuthState::Unauthorized => {
                let http_method = req.method();
                if http_method == salvo::http::Method::GET {
                    // response html
                    let base_url = depot.get::<String>("base_url").ok_or(anyhow::anyhow!("fail to acquire base_url"))?;
                    let tera = depot.get::<Tera>("tera").ok_or(anyhow::anyhow!("fail to acquire tera engine"))?;
                    let mut context = Context::new();
                    context.insert("code", &404);
                    context.insert("msg", "没有权限执行此操作");
                    context.insert("baseUrl", &base_url);
                    let r = tera
                        .render("404.html", &context)
                        .unwrap_or(String::from("error"));
                    res.render(Text::Html(r));
                } else if http_method == salvo::http::Method::POST {
                    let base_url = depot.get::<String>("base_url").ok_or(anyhow::anyhow!("fail to acquire base_url"))?;
                    let r = json!({
                        "code":400,
                        "msg":"没有权限执行此操作",
                        "baseUrl":base_url,
                        "success":0,
                        "message":"没有权限执行此操作",
                    });
                    res.render(Text::Json(r.to_string()))
                }
                ctrl.skip_rest();
                Ok(())
            }
            JwtAuthState::Forbidden => {
                let http_method = req.method();
                if http_method == salvo::http::Method::GET {
                    // response html
                    let base_url = depot.get::<String>("base_url").ok_or(anyhow::anyhow!("fail to acquire base_url"))?;
                    let tera = depot.get::<Tera>("tera").ok_or(anyhow::anyhow!("fail to acquire tera engine"))?;
                    let mut context = Context::new();
                    context.insert("code", &404);
                    context.insert("msg", "没有权限执行此操作");
                    context.insert("baseUrl", &base_url);
                    let r = tera
                        .render("404.html", &context)
                        .unwrap_or(String::from("error"));
                    res.render(Text::Html(r));
                } else if http_method == salvo::http::Method::POST {
                    let base_url = depot.get::<String>("base_url").ok_or(anyhow::anyhow!("fail to acquire base_url"))?;
                    let r = json!({
                        "code":400,
                        "msg":"没有权限执行此操作",
                        "baseUrl":base_url,
                        "success":0,
                        "message":"没有权限执行此操作",
                    });
                    res.render(Text::Json(r.to_string()))
                }
                ctrl.skip_rest();
                Ok(())
            }
        }
    }
}
#[derive(Clone)]
struct SecretKeyForJWT(String);
#[async_trait]
impl Handler for SecretKeyForJWT {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        depot.insert("secret_key", self.0.clone());
        ctrl.call_next(req, depot, res).await;
    }
}

struct Handle404(String, Tera);
#[async_trait]
impl Handler for Handle404 {
    async fn handle(
        &self,
        req: &mut Request,
        _depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        let http_method = req.method();
        if http_method == salvo::http::Method::GET {
            // response html
            let base_url = &self.0;
            let tera = &self.1;
            let mut context = Context::new();
            context.insert("code", &404);
            context.insert("msg", "访问的资源不存在");
            context.insert("baseUrl", &base_url);
            let r = tera
                .render("404.html", &context)
                .unwrap_or(String::from("error"));
            res.render(Text::Html(r));
        } else if http_method == salvo::http::Method::POST {
            let base_url = &self.0;
            let r = json!({
                "code":400,
                "msg":"访问的资源不存在",
                "baseUrl":base_url,
                "success":0,
                "message":"访问的资源不存在",
            });
            res.render(Text::Json(r.to_string()))
        }
        ctrl.skip_rest();
    }
}

struct IsNullFilter;
impl tera::Filter for IsNullFilter {
    fn filter(
        &self,
        value: &sea_orm::JsonValue,
        _args: &std::collections::HashMap<String, sea_orm::JsonValue>,
    ) -> tera::Result<sea_orm::JsonValue> {
        if value.is_null() {
            Ok(json!(true))
        } else {
            Ok(json!(false))
        }
    }
}
use time::{macros::format_description, UtcOffset};
use tracing_subscriber::fmt::time::OffsetTime;

#[tokio::main]
async fn main() {
    let local_time = OffsetTime::new(
        UtcOffset::from_hms(8, 0, 0).unwrap(),
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"),
    );
    let _tracing_guard = if cfg!(debug_assertions) {
        tracing_subscriber::fmt().with_timer(local_time).init();
        None
    } else {
        let file_appender = tracing_appender::rolling::hourly("./logs", "salvo.blog.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_timer(local_time)
            .with_max_level(tracing::Level::INFO)
            .with_writer(non_blocking)
            .init();
        Some(guard)
    };

    match fs::create_dir("./public/upload").await {
        Ok(_) => {}
        Err(e) => {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                panic!("fail to create upload directory");
            }
        }
    };

    let Ok(stream) = fs::read("./config.json").await else{
		panic!("fail to read config file");
	};

    let Ok(content) = std::str::from_utf8(&stream[..]) else{
		panic!("fail to parse the config file content");
	};

    let json_v: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(content);
    let json_v = match json_v {
        Ok(v) => v,
        Err(e) => {
            panic!("{}", e.to_string());
        }
    };

    let base_url = json_v.get("base_url").unwrap().as_str().unwrap().to_owned();
    let database_url = json_v.get("database_url").unwrap().as_str().unwrap();
    let bind_addr = json_v.get("bind_addr").unwrap().as_str().unwrap();
    let secret_key = json_v.get("secret_key").unwrap().as_str().unwrap();

    let mut db_opt = ConnectOptions::new(database_url.to_owned());
    if cfg!(debug_assertions) {
        //println!("in debug mode");
        db_opt.sqlx_logging_level(log::LevelFilter::Info);
    } else {
        //println!("in release mode");
        db_opt
            .sqlx_logging(false)
            .sqlx_logging_level(log::LevelFilter::Error);
    }
    let db = Database::connect(db_opt).await;

    let Ok(db) = db else{
		panic!("db init error");
	};

    let mut tera = match Tera::new("views/**/*.html") {
        Ok(tera) => tera,
        Err(e) => panic!("{}", e.to_string()),
    };

    tera.register_filter("is_null", IsNullFilter);

    // let auth_handler: JwtAuth<JwtClaims> = JwtAuth::new(secret_key.to_owned())
    //     .finders(vec![
    //         // Box::new(HeaderFinder::new()),
    //         Box::new(CookieFinder::new("token")),
    //         // Box::new(CookieFinder::new("jwt_token")),
    //     ])
    //     .response_error(false);
    let auth_handler: JwtAuth<JwtClaims, _> =
        JwtAuth::new(ConstDecoder::from_secret(secret_key.as_bytes()))
            .finders(vec![
                // Box::new(HeaderFinder::new()),
                Box::new(CookieFinder::new("token")),
                // Box::new(CookieFinder::new("jwt_token")),
            ])
            .force_passed(true);

    let router = Router::new()
        .hoop(share_db!(db))
        .hoop(auth_handler)
        .get(home::home);

    let login_router = Router::with_path("login")
        .post(home::login)
        .get(home::render_login_view);
    let home_router = Router::with_path("home/<page>").get(home::home);
    let router = router.push(login_router);
    let router = router.push(home_router);
    let router = router.push(Router::with_path("home/<**>").get(home::home));
    let router = router.push(
        Router::with_path("list/<page>")
            .hoop(AuthorGuardByMethod)
            .get(home::person_list),
    );
    let router = router.push(
        Router::with_path("list/<**>")
            .hoop(AuthorGuardByMethod)
            .get(home::person_list),
    );
    let router = router.push(
        Router::with_path("register")
            .get(home::register)
            .post(home::post_register),
    );

    let router = router.push(Router::with_path("article/<id>").get(home::read_article));

    let router = router.push(
        Router::with_path("add")
            .hoop(AuthorGuardByMethod)
            .get(home::render_add_article_view)
            .post(home::add_article),
    );

    let router = router.push(
        Router::with_path("edit/<id>")
            .hoop(AuthorGuardByMethod)
            .get(home::render_article_edit_view)
            .post(home::edit_article),
    );

    let router = router.push(
        Router::with_path("delete/<id>")
            .hoop(AuthorGuardByMethod)
            .post(home::shadow_article),
    );

    let router = router.push(
        Router::with_path("delcomment/<id>")
            .hoop(AuthorGuardByMethod)
            .post(home::delete_comment),
    );

    let router = router.push(
        Router::with_path("commentedit/<id>")
            .hoop(AuthorGuardByMethod)
            .get(home::edit_comment),
    );

    let router = router.push(
        Router::with_path("editcomment/<id>")
            .hoop(AuthorGuardByMethod)
            .post(home::save_edit_comment),
    );

    let router = router.push(
        Router::with_path("comment/<id>")
            .hoop(AuthorGuardByMethod)
            .post(home::add_comment),
    );

    let router = router.push(
        Router::with_path("profile")
            .hoop(AuthorGuardByMethod)
            .get(home::render_profile_view)
            .post(home::edit_profile),
    );

    let router_static_asserts = Router::with_path("<**path>").get(
        StaticDir::new(["public"])
            .defaults("index.html")
            .auto_list(false),
    );

    let upload_router = Router::with_path("upload")
        .hoop(AuthorGuardByMethod)
        .post(home::upload);

    let router = router.push(upload_router);

    let root_router = Router::new()
        .hoop(TemplateEngineInjection(tera.clone()))
        .hoop(BaseUrlInjector(base_url.clone()))
        .hoop(SecretKeyForJWT(secret_key.to_owned()))
        .push(router)
        .push(router_static_asserts);

    tracing::info!("Listening on {}", bind_addr);
    let acceptor = TcpListener::new(bind_addr).bind().await;
    let service =
        Service::new(root_router).catcher(Catcher::default().hoop(Handle404(base_url, tera)));
    Server::new(acceptor).serve(service).await;
}
