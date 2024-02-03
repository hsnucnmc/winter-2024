qid = null;

let update_captcha = async () => {
    qid = await (await fetch('/api/new-qid')).text();
    document.getElementById('captcha-image').setAttribute('src', '/api/captcha-img/' + qid);
};

let update_list = async () => {
    let users = await (await fetch('/api/users')).text();
    document.getElementById('users-list').innerHTML = 'Current users: (click to update)<br>' + users.split('\n').join('<br>');
};

update_list();