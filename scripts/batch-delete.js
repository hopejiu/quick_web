(function() {
  'use strict';

  // ---------- helpers ----------
  var q = document.querySelector.bind(document);

  function log() {
    var a = ['[batchDelete]'];
    for (var i = 0; i < arguments.length; i++) a.push(arguments[i]);
    console.log.apply(console, a);
  }

  // ---------- guard ----------
  if (window.__batchDeleteInjected) return;
  window.__batchDeleteInjected = true;
  log('injected');

  // ---------- wait for sidebar ----------
  var MAX_RETRIES = 20, retry = 0;

  function wait() {
    if (++retry > MAX_RETRIES) { log('max retries, giving up'); return; }
    var el = q('.ds-scroll-area');
    if (!el) { setTimeout(wait, 300); return; }
    log('ds-scroll-area found after', retry, 'retries');
    setTimeout(main, 200);
  }

  // ---------- main ----------
  function main() {
    log('main()');

    // ---------- user token ----------
    var userToken;
    try {
      var t = JSON.parse(localStorage.getItem('userToken'));
      if (!t || !t.value) { log('no token, not logged in'); return; }
      userToken = t.value;
      log('userToken found');
    } catch (e) {
      log('token parse error:', e);
      return;
    }

    // ---------- inject styles ----------
    var css = [
      '/* batchDelete UI */',
      '.bd-btn{display:inline-flex;align-items:center;justify-content:center;gap:6px;padding:8px 12px;border-radius:8px;font-size:13px;font-weight:500;cursor:pointer;transition:all .2s ease;border:1px solid var(--dsw-alias-border-default,#e5e5ea);background:transparent;color:var(--dsw-alias-text-primary,#1a1a2e);outline:none;user-select:none;overflow:hidden;position:relative;white-space:nowrap}',
      '.bd-btn:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(0,0,0,.05))}',
      '.bd-btn:active{transform:scale(.97)}',
      '.bd-btn-danger{border-color:var(--dsw-alias-state-error-primary,#e54d4d);background:var(--dsw-alias-state-error-primary,#e54d4d);color:#fff}',
      '.bd-btn-danger:hover{filter:brightness(1.12);background:var(--dsw-alias-state-error-primary,#e54d4d)}',
      '.bd-btn-accent{border-color:#c4b5fd;background:#ede9fe;color:#5b21b6}',
      '.bd-btn-accent:hover{background:#ddd6fe}',
      '.bd-btn-disabled{opacity:.4;cursor:not-allowed;filter:none;transform:none}',
      '.bd-btn-disabled:hover{filter:none}',
      '',
      '.bd-container{display:flex;gap:8px;padding:4px 0;overflow:hidden;transition:opacity .2s ease,transform .2s ease}',
      '.bd-container>*{flex:1;min-width:0}',
      '.bd-container-hide{display:none !important}',
      '',
      '@keyframes bdFadeIn{from{opacity:0;transform:translateY(-6px)}to{opacity:1;transform:translateY(0)}}',
      '@keyframes bdSpin{to{transform:rotate(360deg)}}',
      '@keyframes bdScaleIn{from{opacity:0;transform:scale(.8)}to{opacity:1;transform:scale(1)}}',
      '',
      '.bd-cb{-webkit-appearance:none;appearance:none;width:18px;height:18px;min-width:18px;border:2px solid var(--dsw-alias-border-default,#d1d1d6);border-radius:4px;cursor:pointer;margin-right:10px;background:transparent;flex-shrink:0;transition:all .2s;animation:bdFadeIn .25s ease}',
      '.bd-cb:hover{border-color:var(--dsw-alias-brand-primary,#4f6bed)}',
      '.bd-cb:checked{background:var(--dsw-alias-brand-primary,#4f6bed);border-color:var(--dsw-alias-brand-primary,#4f6bed);background-image:url("data:image/svg+xml,%3Csvg xmlns=\'http://www.w3.org/2000/svg\' viewBox=\'0 0 16 16\'%3E%3Cpath fill=\'%23fff\' d=\'M13.5 4.5L6.5 11.5 3 8l-1 1 4.5 4.5 8-8z\'/%3E%3C/svg%3E");background-size:12px;background-position:center;background-repeat:no-repeat}',
      '',
      '.bd-badge{display:inline-flex;align-items:center;justify-content:center;min-width:20px;height:20px;padding:0 6px;border-radius:10px;background:#fff;color:var(--dsw-alias-state-error-primary,#e54d4d);font-size:11px;font-weight:700;line-height:1;margin-left:4px;animation:bdScaleIn .2s ease}',
      '',
      '.bd-progress{position:fixed;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;background:rgba(0,0,0,.45);z-index:999999;animation:bdFadeIn .15s ease}',
      '.bd-progress-card{background:var(--dsw-alias-bg-primary,#fff);border-radius:12px;padding:28px 36px;min-width:200px;text-align:center;box-shadow:0 8px 32px rgba(0,0,0,.15)}',
      '.bd-progress-title{font-size:14px;font-weight:600;color:var(--dsw-alias-text-primary,#1a1a2e);margin-bottom:16px}',
      '.bd-progress-bar-wrap{height:4px;background:var(--dsw-alias-border-default,#e5e5ea);border-radius:2px;overflow:hidden;margin:12px 0 8px}',
      '.bd-progress-bar{height:100%;background:var(--dsw-alias-brand-primary,#4f6bed);border-radius:2px;transition:width .3s ease;width:0%}',
      '.bd-progress-text{font-size:12px;color:var(--dsw-alias-text-secondary,rgba(0,0,0,.45))}',
      '.bd-spinner{display:inline-block;width:20px;height:20px;border:2px solid var(--dsw-alias-border-default,#e5e5ea);border-top-color:var(--dsw-alias-brand-primary,#4f6bed);border-radius:50%;animation:bdSpin .6s linear infinite}',
    ].join('\n');
    var s = document.createElement('style');
    s.textContent = css;
    document.head.appendChild(s);
    log('style injected');

    // ---------- find elements ----------
    var sidebar = q('.ds-scroll-area');
    if (!sidebar) { log('sidebar not found'); return; }
    log('sidebar found');

    var btnNewChat = sidebar.previousSibling;
    if (!btnNewChat) { log('btnNewChat not found'); return; }
    log('btnNewChat found');

    // ---------- create buttons ----------
    var btnBatchDelete = btnNewChat.cloneNode();
    btnBatchDelete.className = '';
    btnBatchDelete.classList.add('bd-btn', 'bd-btn-accent');
    btnBatchDelete.textContent = '\u6279\u91CF\u5220\u9664';

    var btnCancel = btnNewChat.cloneNode();
    btnCancel.className = '';
    btnCancel.classList.add('bd-btn');
    btnCancel.textContent = '\u2715 \u53D6\u6D88';

    var btnReverse = btnNewChat.cloneNode();
    btnReverse.className = '';
    btnReverse.classList.add('bd-btn');
    btnReverse.textContent = '\u21C4 \u53CD\u9009';

    var btnConfirm = btnNewChat.cloneNode();
    btnConfirm.className = '';
    btnConfirm.classList.add('bd-btn', 'bd-btn-danger', 'bd-btn-disabled');
    btnConfirm.innerHTML = '\u2713 \u5220\u9664 <span class="bd-badge" id="bdCount">0</span>';

    var checkboxes = [];
    var container1 = document.createElement('div');
    var container2 = document.createElement('div');

    // row 1: [批量删除] [新对话]
    container1.className = 'bd-container';
    btnNewChat.before(container1);
    container1.appendChild(btnBatchDelete);
    container1.appendChild(btnNewChat);

    // row 2: [取消] [反选] [删除] (hidden initially)
    container2.className = 'bd-container bd-container-hide';
    container1.before(container2);
    container2.appendChild(btnCancel);
    container2.appendChild(btnReverse);
    container2.appendChild(btnConfirm);

    log('buttons created');

    // ---------- selected count ----------
    var countEl = document.getElementById('bdCount');

    function updateCount() {
      var n = 0;
      for (var i = 0; i < checkboxes.length; i++) {
        if (checkboxes[i].checked) n++;
      }
      countEl.textContent = n;
      btnConfirm.classList.toggle('bd-btn-disabled', n === 0);
    }

    // ---------- progress overlay ----------
    var progressEl = null;

    function showProgress(total) {
      progressEl = document.createElement('div');
      progressEl.className = 'bd-progress';
      progressEl.innerHTML = '<div class="bd-progress-card">'
        + '<div class="bd-progress-title">\u6B63\u5728\u5220\u9664</div>'
        + '<div class="bd-spinner"></div>'
        + '<div class="bd-progress-bar-wrap"><div class="bd-progress-bar" id="bdProgressBar"></div></div>'
        + '<div class="bd-progress-text" id="bdProgressText">0 / ' + total + '</div>'
        + '</div>';
      document.body.appendChild(progressEl);
    }

    function updateProgress(done, total) {
      var bar = document.getElementById('bdProgressBar');
      var txt = document.getElementById('bdProgressText');
      if (bar) bar.style.width = (done / total * 100) + '%';
      if (txt) txt.textContent = done + ' / ' + total;
    }

    function hideProgress() {
      if (progressEl && progressEl.parentNode) progressEl.parentNode.removeChild(progressEl);
      progressEl = null;
    }

    // ---------- event listeners ----------
    btnBatchDelete.addEventListener('click', function() {
      log('batchDelete clicked');
      container1.classList.add('bd-container-hide');
      container2.classList.remove('bd-container-hide');
      addCheckboxes();
    });

    btnCancel.addEventListener('click', function() {
      log('cancel clicked');
      container2.classList.add('bd-container-hide');
      container1.classList.remove('bd-container-hide');
      removeCheckboxes();
    });

    btnReverse.addEventListener('click', function() {
      log('reverse clicked');
      for (var i = 0; i < checkboxes.length; i++) {
        checkboxes[i].checked = !checkboxes[i].checked;
      }
      updateCount();
    });

    btnConfirm.addEventListener('click', function() {
      log('confirm clicked');
      confirmDelete();
    });

    // ---------- checkbox functions ----------
    function addCheckboxes() {
      log('addCheckboxes called');
      var inner = sidebar.querySelector('.ds-scroll-area');
      if (!inner) { log('inner scroll area not found'); return; }
      var chats = inner.querySelectorAll('a');
      log('found', chats.length, 'chats');

      for (var i = 0; i < chats.length; i++) {
        var a = chats[i];
        a.style.justifyContent = 'unset';
        a.style.alignItems = 'center';
        var cb = document.createElement('input');
        cb.type = 'checkbox';
        cb.className = 'bd-cb';
        cb.name = a.href.split('/').pop();
        cb.refer = a;
        cb.onclick = function(e) { e.stopPropagation(); };
        cb.addEventListener('change', updateCount);
        a.prepend(cb);
        checkboxes.push(cb);
      }
      updateCount();
      log('added', checkboxes.length, 'checkboxes');
    }

    function removeCheckboxes() {
      log('removeCheckboxes called');
      for (var i = 0; i < checkboxes.length; i++) checkboxes[i].remove();
      checkboxes = [];
      updateCount();
    }

    // ---------- MutationObserver for new chats ----------
    var observer = new MutationObserver(function(mutations) {
      for (var m = 0; m < mutations.length; m++) {
        try {
          for (var n = 0; n < mutations[m].addedNodes.length; n++) {
            var node = mutations[m].addedNodes[n];
            if (node.tagName === 'A') node.parentElement.style.display = '';
          }
        } catch (e) { /* ignore */ }
      }
    });
    var inner = sidebar.querySelector('.ds-scroll-area');
    if (inner && inner.firstChild) {
      observer.observe(inner.firstChild, { childList: true, subtree: true });
      log('observer attached');
    } else {
      log('observer skipped: inner or firstChild missing');
    }

    // ---------- confirm delete ----------
    function confirmDelete() {
      var checked = [];
      for (var i = 0; i < checkboxes.length; i++) {
        if (checkboxes[i].checked) checked.push(checkboxes[i]);
      }
      var total = checked.length;
      log('confirmDelete called, checked:', total, 'total:', checkboxes.length);

      showProgress(total);

      (async function() {
        var done = 0;
        var promises = [];
        for (var i = 0; i < checked.length; i++) {
          var cb = checked[i];
          log('deleting session:', cb.name);
          promises.push(
            fetch('https://chat.deepseek.com/api/v0/chat_session/delete', {
              credentials: 'include',
              headers: {
                'Accept': '*/*',
                'authorization': 'Bearer ' + userToken,
                'content-type': 'application/json'
              },
              body: JSON.stringify({ chat_session_id: cb.name }),
              method: 'POST'
            }).then(function(r) {
              log('delete result for', cb.name, ':', r.status);
              if (r.ok) {
                if (cb.name === location.href.split('/').pop()) {
                  log('current session deleted, creating new chat');
                  btnNewChat.click();
                }
                cb.refer.style.display = 'none';
              }
              done++;
              updateProgress(done, total);
            }).catch(function(e) {
              log('delete failed for', cb.name, ':', e);
              done++;
              updateProgress(done, total);
            })
          );
        }
        var results = await Promise.allSettled(promises);
        var succeeded = 0;
        for (var r = 0; r < results.length; r++) {
          if (results[r].status === 'fulfilled') succeeded++;
        }
        log('all deletions complete:', succeeded, 'succeeded');
        await new Promise(function(r) { setTimeout(r, 400); });
        hideProgress();
      })();

      removeCheckboxes();
      container2.classList.add('bd-container-hide');
      container1.classList.remove('bd-container-hide');
    }
  }

  wait();
})();
