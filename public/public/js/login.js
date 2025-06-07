layui.use(['jquery', 'layer', 'form'], function () {
	var $ = layui.jquery;
	var layer = layui.layer;
	var form = layui.form;



	//$('.login-wrapper').removeClass('layui-hide');

	// 校验两次密码是否一致
	form.verify({

		checkIsSame: function (value) {

			if ($('input[name=password]').val() !== value) {
				return '两次密码输入不一致！';
			}
		}
	});

	// 登录表单提交
	form.on('submit(loginSubmit)', function (obj) {
		obj.field.rememberMe = !!obj.field.remember;
		layer.load(2);
		console.log(obj.field);
		$.post({url:`${window.baseUrl}login`,headers:{"Content-type":"application/x-www-form-urlencoded"}}, obj.field, function (res) {
			if (200 === res.code) {
				console.log(res);
				layer.msg('登录成功', { icon: 1, time: 1500 }, function () {
					if (obj.field.rememberMe === true) {
						document.cookie = `token=${res.token};expires=${new Date(new Date().setDate(new Date().getDate() + 30)).toUTCString()};Path=${res.baseUrl}`;
					} else {
						document.cookie = `token=${res.token};Path=${res.baseUrl}`;
					}
					location.replace(`${window.baseUrl}home/1`);
				});
			} else {
				layer.closeAll('loading');
				layer.msg(res.msg, { icon: 5 });
				$('img.login-captcha').trigger('click');
			}
		}, 'JSON');
		return false;
	});

	// 注册表单提交
	form.on('submit(regSubmit)', function (obj) {
		layer.load(2);
		$.post({url:`${window.baseUrl}register`,headers:{"Content-type":"application/x-www-form-urlencoded"}}, obj.field, function (res) {
			console.log(res)
			if (200 === res.code) {
				//console.log(res);
				layer.msg(res.msg, { icon: 1, time: 1500 }, function () {
					//console.log("abc:", `token=${res.token};expires=${new Date(new Date().setDate(new Date().getDate() + 10)).toUTCString()}`);
					document.cookie = `token=${res.token};expires=${new Date(new Date().setDate(new Date().getDate() + 30)).toUTCString()};Path=${res.baseUrl}`;
					location.replace(`${window.baseUrl}home/1`);
				});
			} else {
				layer.closeAll('loading');
				layer.msg(res.msg, { icon: 5 });
				$('img.login-captcha').trigger('click');
			}
		},'JSON').fail(function(res){
			layer.closeAll('loading');
			layer.msg(res.responseJSON.msg, { icon: 5 });
		});
		return false;
	});

	//找回密码
	form.on('submit(forgetSubmit)', function (obj) {
		layer.load(2);
		$.post({url:`${window.baseUrl}forget`,headers:{"Content-type":"application/x-www-form-urlencoded"}}, obj.field, function (res) {
			console.log(res)
			if (200 === res.code) {
				//console.log(res);
				layer.msg(res.msg, { icon: 1, time: 1500 }, function () {
					location.replace(`${window.baseUrl}login`);
				});
			} else {
				layer.closeAll('loading');
				layer.msg(res.msg, { icon: 5 });
				$('img.login-captcha').trigger('click');
			}
		},'JSON').fail(function(res){
			layer.closeAll('loading');
			layer.msg(res.responseJSON.msg, { icon: 5 });
		});
		return false;
	});

	/* 图形验证码 */
	var captchaUrl = '/assets/captcha';
	$('img.login-captcha').click(function () {
		this.src = captchaUrl + '?t=' + (new Date).getTime();
	}).trigger('click');

	//邮箱验证
	var checkCode = "";//验证码

	$("#sendCheckCode").click(function (e) {
		e.stopPropagation();
		e.preventDefault(); 
		var email = $("#email").val();
		if (email == null || email == "") {
			layer.msg("请输入邮箱账号");
			return;
		}
		var index = layer.open({
			type: 3,
			content: "邮件发送中..."
		});
		$.ajax({
			url: `${window.baseUrl}sendcode`,
			headers:{"Content-type":"application/x-www-form-urlencoded"},
			type: 'POST', 
			data:`email=${email}`,  //970252187@qq.com
			success: function (res) {
				console.log(res);
				if (res.code==200) {
					layer.close(index);
					layer.msg("已发送");
					countDown();
				} else {
					layer.alert("获取失败，请重新获取")
				}
			}
		});
	});

	var maxTime = 60;
	function countDown() {
		if (maxTime == 0) {
			checkCode = "";
			$("#sendCheckCode").removeClass("layui-btn-disabled");
			$("#sendCheckCode").removeAttr("disabled")
			$("#sendCheckCode").html("获取验证码");
			maxTime = 60;
		} else {
			$("#sendCheckCode").attr("disabled", "disabled");
			$("#sendCheckCode").addClass("layui-btn-disabled");
			form.render();
			$("#sendCheckCode").html(maxTime + "秒后重新获取");
			maxTime--;
			setTimeout(countDown, 1000);
		}
	}

});
