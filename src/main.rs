use salvo::prelude::*;
use sea_orm::{Database, DatabaseConnection};
mod home;

use salvo::serve_static::StaticDir;

use serde_json::json;
use tokio::fs;

use tera::{Context, Tera};

use salvo::jwt_auth::CookieFinder;

use home::SECRET_KEY;

use home::JwtClaims;

use home::UniformError;

use home::ConverOptionToResult;

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
struct AuthorGuard(bool);
#[async_trait]
impl Handler for AuthorGuard {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        match depot.jwt_auth_state() {
            JwtAuthState::Authorized => {
                ctrl.call_next(req, depot, res).await;
            }
            JwtAuthState::Unauthorized => {
                if self.0 == true { // response html
                    let base_url = depot.get::<String>("base_url").unwrap();
                    let tera = depot.get::<Tera>("tera").unwrap();
                    let mut context = Context::new();
                    context.insert("code", &404);
                    context.insert("msg", "没有权限执行此操作");
                    context.insert("baseUrl", &base_url);
                    let r = tera.render("404.html", &context).unwrap_or(String::from("error"));
                    res.render(Text::Html(r));
                } else if req.method() == salvo::http::Method::POST {
                    let base_url = depot.get::<String>("base_url").unwrap();
                    let r = json!({
                        "code":400,
                        "msg":"没有权限执行此操作",
                        "baseUrl":base_url
                    });
                    res.render(Text::Json(r.to_string()))
                }
                ctrl.skip_rest();
            }
            JwtAuthState::Forbidden => {
                ctrl.skip_rest();
            },
        }
    }
}

async fn auth_guard(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
    ctrl: &mut FlowCtrl,
) -> Result<(), UniformError> {
    println!("------------------------auth guard");
    match depot.jwt_auth_state() {
        JwtAuthState::Authorized => {
            ctrl.call_next(req, depot, res).await;
        }
        JwtAuthState::Unauthorized => {
            if req.method() == salvo::http::Method::GET {
                let base_url = depot.get::<String>("base_url").to_result()?;
                let tera = depot.get::<Tera>("tera").to_result()?;
                let mut context = Context::new();
                context.insert("code", &404);
                context.insert("msg", "没有权限执行此操作");
                context.insert("baseUrl", &base_url);
                let r = tera.render("404.html", &context)?;
                res.render(Text::Html(r));
            } else if req.method() == salvo::http::Method::POST {
                let base_url = depot.get::<String>("base_url").to_result()?;
                let r = json!({
                    "code":400,
                    "msg":"没有权限执行此操作",
                    "baseUrl":base_url
                });
                res.render(Text::Json(r.to_string()))
            }
            ctrl.skip_rest();
        }
        JwtAuthState::Forbidden => todo!(),
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

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

    let db = Database::connect(database_url).await;
    let Ok(db) = db else{
		panic!("db init error");
	};

    let tera = match Tera::new("views/**/*.html") {
        Ok(tera) => tera,
        Err(e) => panic!("{}", e.to_string()),
    };

    let auth_handler: JwtAuth<JwtClaims> = JwtAuth::new(SECRET_KEY.to_owned())
        .with_finders(vec![
            // Box::new(HeaderFinder::new()),
            Box::new(CookieFinder::new("token")),
            // Box::new(CookieFinder::new("jwt_token")),
        ])
        .with_response_error(false);

    let router = Router::new().hoop(share_db!(db)).hoop(auth_handler);

    let login_router = Router::with_path("login").post(home::login);
    let home_router = Router::with_path("home/<page>").get(home::home);
    let router = router.push(login_router);
    let router = router.push(home_router);
    let router = router.push(
        Router::with_path("list/<page>")
            .hoop(AuthorGuard(true))
            .get(home::person_list),
    );
    let router = router.push(Router::with_path("test").get(home::test_duplicate));

    let router_static_asserts = Router::with_path("<**path>").get(
        StaticDir::new(["public"])
            .with_defaults("index.html")
            .with_listing(true),
    );

    let root_router = Router::new()
        .hoop(TemplateEngineInjection(tera))
        .hoop(BaseUrlInjector(base_url))
        .push(router)
        .push(router_static_asserts);

    tracing::info!("Listening on 0.0.0.0:8080");

    Server::new(TcpListener::bind("0.0.0.0:8080"))
        .serve(root_router)
        .await;
}
