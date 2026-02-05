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
	var canvas = document.getElementById('canvas-banner'),
	ctx = canvas.getContext('2d'),
	WIDTH,
	HEIGHT,
	particles = [],
	particleCount = 60,
	connectionDistance = 150,
	mouse = {
		x: null,
		y: null,
		radius: 150
	},
	hue = 0;

	WIDTH = window.document.body.clientWidth;
	if (screen.width >= 992) {
		HEIGHT = window.innerHeight * 1 / 2;
	} else {
		HEIGHT = window.innerHeight * 2 / 7;
	}

	canvas.setAttribute("width", WIDTH);
	canvas.setAttribute("height", HEIGHT);

	// Particle class with modern floating effect
	function Particle() {
		this.x = Math.random() * WIDTH;
		this.y = Math.random() * HEIGHT;
		this.size = Math.random() * 3 + 1;
		this.speedX = Math.random() * 1 - 0.5;
		this.speedY = Math.random() * 1 - 0.5;
		this.opacity = Math.random() * 0.5 + 0.3;
	}

	Particle.prototype.update = function() {
		this.x += this.speedX;
		this.y += this.speedY;

		// Bounce off walls
		if (this.x > WIDTH || this.x < 0) {
			this.speedX = -this.speedX;
		}
		if (this.y > HEIGHT || this.y < 0) {
			this.speedY = -this.speedY;
		}

		// Mouse interaction
		if (mouse.x !== null && mouse.y !== null) {
			let dx = mouse.x - this.x;
			let dy = mouse.y - this.y;
			let distance = Math.sqrt(dx * dx + dy * dy);
			if (distance < mouse.radius) {
				let force = (mouse.radius - distance) / mouse.radius;
				let directionX = dx / distance;
				let directionY = dy / distance;
				this.x -= directionX * force * 3;
				this.y -= directionY * force * 3;
			}
		}
	}

	Particle.prototype.draw = function() {
		ctx.fillStyle = 'rgba(255, 255, 255, ' + this.opacity + ')';
		ctx.beginPath();
		ctx.arc(this.x, this.y, this.size, 0, Math.PI * 2);
		ctx.fill();
	}

	// Initialize particles
	function init() {
		particles = [];
		for (let i = 0; i < particleCount; i++) {
			particles.push(new Particle());
		}
	}

	// Connect particles with lines
	function connect() {
		for (let a = 0; a < particles.length; a++) {
			for (let b = a + 1; b < particles.length; b++) {
				let dx = particles[a].x - particles[b].x;
				let dy = particles[a].y - particles[b].y;
				let distance = Math.sqrt(dx * dx + dy * dy);

				if (distance < connectionDistance) {
					let opacity = 1 - (distance / connectionDistance);
					ctx.strokeStyle = 'rgba(102, 126, 234, ' + opacity * 0.3 + ')';
					ctx.lineWidth = 1;
					ctx.beginPath();
					ctx.moveTo(particles[a].x, particles[a].y);
					ctx.lineTo(particles[b].x, particles[b].y);
					ctx.stroke();
				}
			}
		}
	}

	// Animation loop
	function animate() {
		// Create gradient background effect
		let gradient = ctx.createLinearGradient(0, 0, WIDTH, HEIGHT);
		gradient.addColorStop(0, 'rgba(102, 126, 234, 0.05)');
		gradient.addColorStop(0.5, 'rgba(118, 75, 162, 0.05)');
		gradient.addColorStop(1, 'rgba(102, 126, 234, 0.05)');
		ctx.fillStyle = gradient;
		ctx.fillRect(0, 0, WIDTH, HEIGHT);

		// Update and draw particles
		for (let i = 0; i < particles.length; i++) {
			particles[i].update();
			particles[i].draw();
		}

		// Draw connections
		connect();

		// Increment hue for color cycling
		hue += 0.2;
		if (hue > 360) hue = 0;

		requestAnimationFrame(animate);
	}

	// Mouse event handlers
	canvas.addEventListener('mousemove', function(e) {
		mouse.x = e.offsetX;
		mouse.y = e.offsetY;
	});

	canvas.addEventListener('mouseleave', function() {
		mouse.x = null;
		mouse.y = null;
	});

	// Handle touch events for mobile
	canvas.addEventListener('touchmove', function(e) {
		e.preventDefault();
		let rect = canvas.getBoundingClientRect();
		mouse.x = e.touches[0].clientX - rect.left;
		mouse.y = e.touches[0].clientY - rect.top;
	});

	canvas.addEventListener('touchend', function() {
		mouse.x = null;
		mouse.y = null;
	});

	init();
	animate();
}





//监听窗口大小改变
window.addEventListener("resize", resizeCanvas, false);

//窗口大小改变时改变canvas宽度并重新绘制
function resizeCanvas() {
    var canvas = document.getElementById('canvas-banner');
	canvas.width = window.document.body.clientWidth;
	if (screen.width >= 992) {
		canvas.height = window.innerHeight * 1 / 2;
	} else {
		canvas.height = window.innerHeight * 2 / 7;
	}
	// Redraw canvas after resize
	DrawCanvas();
}