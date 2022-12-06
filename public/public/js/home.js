/*

@Name：不落阁整站模板源码 
@Author：Absolutely 
@Site：http://www.lyblogs.cn

*/

layui.use('jquery', function () {
    var $ = layui.jquery;
    $(function () {
        //播放公告
        playAnnouncement(3000);
    });
    function playAnnouncement(interval) {
        var index = 0;
        var $announcement = $('.home-tips-container>span');
        //自动轮换
        setInterval(function () {
            index++;    //下标更新
            if (index >= $announcement.length) {
                index = 0;
            }
            $announcement.eq(index).stop(true, true).fadeIn().siblings('span').fadeOut();  //下标对应的图片显示，同辈元素隐藏
        }, interval);
    }
    //画canvas
    DrawCanvas();
});

// function DrawCanvas() {
//     var $ = layui.jquery;
//     var canvas = document.getElementById('canvas-banner');
//     canvas.width = window.document.body.clientWidth;    //需要重新设置canvas宽度，因为dom加载完毕后有可能没有滚动条
//     var ctx = canvas.getContext('2d');

//     ctx.strokeStyle = (new Color(150)).style;

//     var dotCount = 20; //圆点数量
//     var dotRadius = 70; //产生连线的范围
//     var dotDistance = 70;   //产生连线的最小距离
//     var screenWidth = screen.width;
//     if (screenWidth >= 768 && screenWidth < 992) {
//         dotCount = 130;
//         dotRadius = 100;
//         dotDistance = 60;
//     } else if (screenWidth >= 992 && screenWidth < 1200) {
//         dotCount = 140;
//         dotRadius = 140;
//         dotDistance = 70;
//     } else if (screenWidth >= 1200 && screenWidth < 1700) {
//         dotCount = 140;
//         dotRadius = 150;
//         dotDistance = 80;
//     } else if (screenWidth >= 1700) {
//         dotCount = 200;
//         dotRadius = 150;
//         dotDistance = 80;
//     } 
//     //默认鼠标位置 canvas 中间
//     var mousePosition = {
//         x: 50 * canvas.width / 100,
//         y: 50 * canvas.height / 100
//     };
//     //小圆点
//     var dots = {
//         count: dotCount,
//         distance: dotDistance,
//         d_radius: dotRadius,
//         array: []
//     };

//     function colorValue(min) {
//         return Math.floor(Math.random() * 255 + min);
//     }

//     function createColorStyle(r, g, b) {
//         return 'rgba(' + r + ',' + g + ',' + b + ', 0.8)';
//     }

//     function mixComponents(comp1, weight1, comp2, weight2) {
//         return (comp1 * weight1 + comp2 * weight2) / (weight1 + weight2);
//     }

//     function averageColorStyles(dot1, dot2) {
//         var color1 = dot1.color,
//             color2 = dot2.color;

//         var r = mixComponents(color1.r, dot1.radius, color2.r, dot2.radius),
//             g = mixComponents(color1.g, dot1.radius, color2.g, dot2.radius),
//             b = mixComponents(color1.b, dot1.radius, color2.b, dot2.radius);
//         return createColorStyle(Math.floor(r), Math.floor(g), Math.floor(b));
//     }

//     function Color(min) {
//         min = min || 0;
//         this.r = colorValue(min);
//         this.g = colorValue(min);
//         this.b = colorValue(min);
//         this.style = createColorStyle(this.r, this.g, this.b);
//     }

//     function Dot() {
//         this.x = Math.random() * canvas.width;
//         this.y = Math.random() * canvas.height;

//         this.vx = -.5 + Math.random();
//         this.vy = -.5 + Math.random();

//         this.radius = Math.random() * 2;

//         this.color = new Color();
//     }

//     Dot.prototype = {
//         draw: function () {
//             ctx.beginPath();
//             ctx.fillStyle = "#fff";
//             ctx.arc(this.x, this.y, this.radius, 0, Math.PI * 2, false);
//             ctx.fill();
//         }
//     };

//     function createDots() {
//         for (i = 0; i < dots.count; i++) {
//             dots.array.push(new Dot());
//         }
//     }

//     function moveDots() {
//         for (i = 0; i < dots.count; i++) {

//             var dot = dots.array[i];

//             if (dot.y < 0 || dot.y > canvas.height) {
//                 dot.vx = dot.vx;
//                 dot.vy = -dot.vy;
//             }
//             else if (dot.x < 0 || dot.x > canvas.width) {
//                 dot.vx = -dot.vx;
//                 dot.vy = dot.vy;
//             }
//             dot.x += dot.vx;
//             dot.y += dot.vy;
//         }
//     }

//     function connectDots1() {
//         var pointx = mousePosition.x;
//         for (i = 0; i < dots.count; i++) {
//             for (j = 0; j < dots.count; j++) {
//                 i_dot = dots.array[i];
//                 j_dot = dots.array[j];

//                 if ((i_dot.x - j_dot.x) < dots.distance && (i_dot.y - j_dot.y) < dots.distance && (i_dot.x - j_dot.x) > -dots.distance && (i_dot.y - j_dot.y) > -dots.distance) {
//                     if ((i_dot.x - pointx) < dots.d_radius && (i_dot.y - mousePosition.y) < dots.d_radius && (i_dot.x - pointx) > -dots.d_radius && (i_dot.y - mousePosition.y) > -dots.d_radius) {
//                         ctx.beginPath();
//                         ctx.strokeStyle = averageColorStyles(i_dot, j_dot);
//                         ctx.moveTo(i_dot.x, i_dot.y);
//                         ctx.lineTo(j_dot.x, j_dot.y);
//                         ctx.stroke();
//                         ctx.closePath();
//                     }
//                 }
//             }
//         }
//     }

//     function drawDots() {
//         for (i = 0; i < dots.count; i++) {
//             var dot = dots.array[i];
//             dot.draw();
//         }
//     }

//     function animateDots() {
//         ctx.clearRect(0, 0, canvas.width, canvas.height);
//         moveDots();
//         connectDots1()
//         drawDots();

//         requestAnimationFrame(animateDots);
//     }
//     //鼠标在canvas上移动
//     $('canvas').on('mousemove', function (e) {
//         mousePosition.x = e.pageX;
//         mousePosition.y = e.pageY;
//     });

//     //鼠标移出canvas
//     $('canvas').on('mouseleave', function (e) {
//         mousePosition.x = canvas.width / 2;
//         mousePosition.y = canvas.height / 2;
//     });

//     createDots();

//     requestAnimationFrame(animateDots);
// }



function DrawCanvas(){
	var canvas  = document.getElementById('canvas-banner'),
	ctx = canvas.getContext('2d'),
	WIDTH,
	HEIGHT,
	mouseMoving = false,
	mouseMoveChecker,
	mouseX,
	mouseY,
	stars = [],
	initStarsPopulation = 80,
	dots = [],
	dotsMinDist = 2,
	maxDistFromCursor = 50;
	WIDTH = window.document.body.clientWidth;
	if (screen.width >= 992) {
		HEIGHT = window.innerHeight * 1 / 3;
	} else {
		HEIGHT = window.innerHeight * 2 / 7;
	}                  

	canvas.setAttribute("width", WIDTH);
	canvas.setAttribute("height", HEIGHT);
	ctx.strokeStyle = "white";
	ctx.shadowColor = "white";
	for (var i = 0; i < initStarsPopulation; i++) {
		stars[i] = new Star(i, Math.floor(Math.random()*WIDTH), Math.floor(Math.random()*HEIGHT));
		//stars[i].draw();
	}
	ctx.shadowBlur = 0;

	function Star(id, x, y){
		this.id = id;
		this.x = x;
		this.y = y;
		this.r = Math.floor(Math.random()*2)+1;
		var alpha = (Math.floor(Math.random()*10)+1)/10/2;
		this.color = "rgba(255,255,255,"+alpha+")";
	}
	
	Star.prototype.draw = function() {
		ctx.fillStyle = this.color;
		ctx.shadowBlur = this.r * 2;
		ctx.beginPath();
		ctx.arc(this.x, this.y, this.r, 0, 2 * Math.PI, false);
		ctx.closePath();
		ctx.fill();
	}
	
	Star.prototype.move = function() {
		this.y -= .15;
		if (this.y <= -10) this.y = HEIGHT + 10;
		this.draw();
	}
	
	Star.prototype.die = function() {
		stars[this.id] = null;
		delete stars[this.id];
	}
	
	
	function Dot(id, x, y, r) {
		this.id = id;
		this.x = x;
		this.y = y;
		this.r = Math.floor(Math.random()*5)+1;
		this.maxLinks = 2;
		this.speed = .5;
		this.a = .5;
		this.aReduction = .005;
		this.color = "rgba(255,255,255,"+this.a+")";
		this.linkColor = "rgba(255,255,255,"+this.a/4+")";
	
		this.dir = Math.floor(Math.random()*140)+200;
	}
	
	Dot.prototype.draw = function() {
		ctx.fillStyle = this.color;
		ctx.shadowBlur = this.r * 2;
		ctx.beginPath();
		ctx.arc(this.x, this.y, this.r, 0, 2 * Math.PI, false);
		ctx.closePath();
		ctx.fill();
	}
	
	Dot.prototype.link = function() {
		if (this.id == 0) return;
		var previousDot1 = getPreviousDot(this.id, 1);
		var previousDot2 = getPreviousDot(this.id, 2);
		var previousDot3 = getPreviousDot(this.id, 3);
		if (!previousDot1) return;
		ctx.strokeStyle = this.linkColor;
		ctx.moveTo(previousDot1.x, previousDot1.y);
		ctx.beginPath();
		ctx.lineTo(this.x, this.y);
		if (previousDot2 != false) ctx.lineTo(previousDot2.x, previousDot2.y);
		if (previousDot3 != false) ctx.lineTo(previousDot3.x, previousDot3.y);
		ctx.stroke();
		ctx.closePath();
	}
	
	function getPreviousDot(id, stepback) {
		if (id == 0 || id - stepback < 0) return false;
		if (typeof dots[id - stepback] != "undefined") return dots[id - stepback];
		else return false;//getPreviousDot(id - stepback);
	}
	
	Dot.prototype.move = function() {
		this.a -= this.aReduction;
		if (this.a <= 0) {
			this.die();
			return
		}
		this.color = "rgba(255,255,255,"+this.a+")";
		this.linkColor = "rgba(255,255,255,"+this.a/4+")";
		this.x = this.x + Math.cos(degToRad(this.dir))*this.speed,
		this.y = this.y + Math.sin(degToRad(this.dir))*this.speed;
	
		this.draw();
		this.link();
	}
	
	Dot.prototype.die = function() {
		dots[this.id] = null;
		delete dots[this.id];
	}
	const drawIfMouseMoving = function(){
		if (!mouseMoving) return;
	
		if (dots.length == 0) {
			dots[0] = new Dot(0, mouseX, mouseY);
			dots[0].draw();
			return;
		}
	
		var previousDot = getPreviousDot(dots.length, 1);
		var prevX = previousDot.x; 
		var prevY = previousDot.y; 
	
		var diffX = Math.abs(prevX - mouseX);
		var diffY = Math.abs(prevY - mouseY);
	
		if (diffX < dotsMinDist || diffY < dotsMinDist) return;
	
		var xVariation = Math.random() > .5 ? -1 : 1;
		xVariation = xVariation*Math.floor(Math.random()*maxDistFromCursor)+1;
		var yVariation = Math.random() > .5 ? -1 : 1;
		yVariation = yVariation*Math.floor(Math.random()*maxDistFromCursor)+1;
		dots[dots.length] = new Dot(dots.length, mouseX+xVariation, mouseY+yVariation);
		dots[dots.length-1].draw();
		dots[dots.length-1].link();
	}

	const animate = ()=>{
		ctx.clearRect(0, 0, WIDTH, HEIGHT);

		for (var i in stars) {
			stars[i].move();
		}
		for (var i in dots) {
			dots[i].move();
		}
		drawIfMouseMoving();
		requestAnimationFrame(animate);
	}
	animate();

	window.onmousemove = function(e){
		mouseMoving = true;
		mouseX = e.clientX;
		mouseY = e.clientY;
		clearInterval(mouseMoveChecker);
		mouseMoveChecker = setTimeout(function() {
			mouseMoving = false;
		}, 100);
	}
}





//setInterval(drawIfMouseMoving, 17);

function degToRad(deg) {
	return deg * (Math.PI / 180);
}

//监听窗口大小改变
window.addEventListener("resize", resizeCanvas, false);

//窗口大小改变时改变canvas宽度
function resizeCanvas() {
    var canvas = document.getElementById('canvas-banner');
	canvas.width = window.document.body.clientWidth ;//减去滚动条的宽度
	if (screen.width >= 992) {
		canvas.height = window.innerHeight * 1 / 3;
	} else {
		canvas.height = window.innerHeight * 2 / 7;
	}
}