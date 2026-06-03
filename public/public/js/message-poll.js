// Shared inline message polling utility
// Exposes window.MessagePoll with helpers used by all pages that have a
// .message-inline-trigger button and a #messageDialogContent container.
(function (window, $) {
	function escapeHtml(str) {
		return String(str || "").replace(/[&<>"'`\/]/g, function (s) {
			return ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;", "`": "&#96;", "/": "&#x2F;" })[s];
		});
	}

	function renderInlineMessages(messages) {
		var baseUrl = window.baseUrl || "";
		var items = messages || [];
		var html = '<div class="message-dialog-body">';
		if (items.length === 0) {
			html += '<div class="message-inline-empty">暂无消息</div>';
		} else {
			items.forEach(function (msg) {
				var unreadClass = Number(msg.is_read) === 0 ? " message-inline-item-unread" : "";
				var fromUser = escapeHtml(msg.from_user_name || "未知用户");
				var articleTitle = escapeHtml(msg.article_title || "文章已删除");
				var msgId = encodeURIComponent(String(msg.id || ""));
				var articleId = encodeURIComponent(String(msg.article_id || ""));
				var commentId = encodeURIComponent(String(msg.comment_id || ""));
				html += '<a class="message-inline-item' + unreadClass + ' inline-jump-btn"' +
					' data-id="' + msgId + '"' +
					' href="' + baseUrl + 'article/' + articleId + '#comment-' + commentId + '">' +
					'<div class="message-inline-title">@' + fromUser + '</div>' +
					'<div class="message-inline-text">在《' + articleTitle + '》提到了你</div>' +
					'</a>';
			});
		}
		html += '<a class="message-inline-more" href="' + baseUrl + 'messages">查看完整消息历史</a></div>';
		return html;
	}

	function updateInlineBadge(unreadCount) {
		var trigger = $(".message-inline-trigger");
		trigger.find(".inline-unread-badge").remove();
		if (Number(unreadCount) > 0) {
			trigger.append('<em class="inline-unread-badge">' + Number(unreadCount) + '</em>');
		}
	}

	function refreshMessageSummary() {
		var baseUrl = window.baseUrl || "";
		return $.get(baseUrl + "message/summary", function (r) {
			if (r.code === 200) {
				updateInlineBadge(r.unread_count || 0);
				$("#messageDialogContent").html(renderInlineMessages(r.recent_messages || []));
			}
		}, "JSON").fail(function () {
			// silent fail — do not interrupt the user's workflow
		});
	}

	// Sets up the trigger button click handler and starts polling.
	// Must be called after layui.use('layer', ...) so that `layer` is available.
	function initInlineMessagePoll(layer) {
		if ($(".message-inline-trigger").length === 0) {
			return;
		}
		var baseUrl = window.baseUrl || "";
		var messagePollTimer = null;

		$(".message-inline-trigger").off("click.messagePoll").on("click.messagePoll", function () {
			layer.open({
				type: 1,
				title: "站内信",
				closeBtn: 1,
				area: ["360px", "400px"],
				content: $("#messageDialogContent").html(),
				success: function (layero) {
					layero.find(".inline-jump-btn").off("click.inlineJump").on("click.inlineJump", function (e) {
						e.preventDefault();
						var link = $(this);
						var id = link.attr("data-id");
						var href = link.attr("href");
						$.post(baseUrl + "message/read/" + id, {}, function (r) {
							if (r.code === 200) {
								layer.closeAll();
								window.location.href = href;
							} else {
								layer.msg(r.msg || "标记已读失败", { icon: 5 });
							}
						}, "JSON").fail(function () {
							layer.msg("网络错误，未能标记已读", { icon: 5 });
						});
					});
				}
			});
		});

		refreshMessageSummary();
		messagePollTimer = setInterval(refreshMessageSummary, 10000);
		$(window).off("beforeunload.messagePoll").on("beforeunload.messagePoll", function () {
			if (messagePollTimer) {
				clearInterval(messagePollTimer);
			}
		});
	}

	window.MessagePoll = {
		escapeHtml: escapeHtml,
		renderInlineMessages: renderInlineMessages,
		updateInlineBadge: updateInlineBadge,
		refreshMessageSummary: refreshMessageSummary,
		initInlineMessagePoll: initInlineMessagePoll
	};
}(window, jQuery));
