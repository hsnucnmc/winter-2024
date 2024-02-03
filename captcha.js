qid = null;

let update_captcha = async () => {
    qid = await (await fetch('/api/new-qid')).text();
    document.getElementById('captcha-image').setAttribute('src', '/api/captcha-img/' + qid);
    document.getElementById('createuser-captcha-ans').removeAttribute('disabled');
};

let update_list = async () => {
    let users = await (await fetch('/api/users')).text();
    document.getElementById('users-list').innerHTML = 'Current users: (click to update)<br>' + users.split('\n').join('<br>');
};

let submit_user = async () => {
    let request = new Object;
    request.username = document.getElementById("createuser-name").value;
    request.captcha_qid = Number(qid);
    request.captcha_ans = document.getElementById("createuser-captcha-ans").value;
    let response = await fetch("/api/submit", {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify(request)
    });
    if (response.status==201){
        update_list();
        document.getElementById("createuser-name").value = "";
        document.getElementById("createuser-captcha-ans").value = "";
        document.getElementById("createuser-result").innerText = "User Created!";
    }
    else {
        document.getElementById("createuser-result").innerHTML = "Failed: " + response.statusText;
    }
    document.getElementById('captcha-image').setAttribute('src', 'youarebot.png');
    document.getElementById('createuser-captcha-ans').setAttribute("disabled", "");
    qid=null;
}

document.getElementById("createuser-form").addEventListener("submit", function (e) {
    e.preventDefault();
    submit_user();
});

update_list();