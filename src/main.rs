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
#[async_trait]
impl Handler for AuthorGuardByMethod {
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
				let http_method = req.method() ;
                if http_method == salvo::http::Method::GET { // response html
                    let base_url = depot.get::<String>("base_url").unwrap();
                    let tera = depot.get::<Tera>("tera").unwrap();
                    let mut context = Context::new();
                    context.insert("code", &404);
                    context.insert("msg", "没有权限执行此操作");
                    context.insert("baseUrl", &base_url);
                    let r = tera.render("404.html", &context).unwrap_or(String::from("error"));
                    res.render(Text::Html(r));
                } else if http_method == salvo::http::Method::POST {
                    let base_url = depot.get::<String>("base_url").unwrap();
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
            }
            JwtAuthState::Forbidden => {
                ctrl.skip_rest();
            },
        }
    }
}



#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

	match fs::create_dir("./public/upload").await{
		Ok(_) => {},
		Err(e) => {
			if e.kind() != std::io::ErrorKind::AlreadyExists{
				panic!("fail to create upload directory");
			}
		},
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
            .hoop(AuthorGuardByMethod)
            .get(home::person_list),
    );
    let router = router.push(Router::with_path("register").get(home::register).post(home::post_register));

    let router = router.push(Router::with_path("article/<id>").get(home::read_article));

	let router = router.push(Router::with_path("add").hoop(AuthorGuardByMethod).get(home::render_add_article_view).post(home::add_article));

	let router = router.push(Router::with_path("edit/<id>").hoop(AuthorGuardByMethod).get(home::render_article_edit_view).post(home::edit_article));

	let router = router.push(Router::with_path("delete/<id>").hoop(AuthorGuardByMethod).post(home::shadow_article));

	let router = router.push(Router::with_path("delcomment/<id>").hoop(AuthorGuardByMethod).post(home::delete_comment));

	let router = router.push(Router::with_path("commentedit/<id>").hoop(AuthorGuardByMethod).get(home::edit_comment));
	
	let router = router.push(Router::with_path("editcomment/<id>").hoop(AuthorGuardByMethod).post(home::save_edit_comment));

	let router = router.push(Router::with_path("comment/<id>").hoop(AuthorGuardByMethod).post(home::add_comment));

	let router = router.push(Router::with_path("profile").hoop(AuthorGuardByMethod).get(home::render_profile_view).post(home::edit_profile));



    let router_static_asserts = Router::with_path("<**path>").get(
        StaticDir::new(["public"])
            .with_defaults("index.html")
            .with_listing(true),
    );

	let upload_router = Router::with_path("upload").hoop(AuthorGuardByMethod).post(home::upload);

	let router = router.push(upload_router);

    let root_router = Router::new()
        .hoop(TemplateEngineInjection(tera))
        .hoop(BaseUrlInjector(base_url))
        .push(router)
        .push(router_static_asserts);

    tracing::info!("Listening on {}",bind_addr);

    Server::new(TcpListener::bind(bind_addr))
        .serve(root_router)
        .await;
}
