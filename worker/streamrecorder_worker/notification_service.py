from __future__ import annotations

import base64
import re
import smtplib
from email.header import Header
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText
from typing import Any

import httpx


class NotificationService:
    def __init__(self) -> None:
        self.headers = {"Content-Type": "application/json"}

    async def _async_post(
        self, url: str, json_data: dict[str, Any], proxy: str | None = None
    ) -> dict[str, Any]:
        try:
            async with httpx.AsyncClient(proxy=proxy) as client:
                response = await client.post(url, json=json_data, headers=self.headers)
                response.raise_for_status()
                if response.headers.get("content-type", "").startswith(
                    "application/json"
                ):
                    return response.json()
                return {"success": True}
        except Exception as exc:  # noqa: BLE001
            return {"error": str(exc)}

    async def send_to_dingtalk(
        self,
        url: str,
        content: str,
        number: str | None = None,
        is_atall: bool = False,
    ) -> dict[str, list[str]]:
        results = {"success": [], "error": []}
        api_list = [u.strip() for u in url.replace("，", ",").split(",") if u.strip()]
        for api in api_list:
            json_data = {
                "msgtype": "text",
                "text": {"content": content},
                "at": {"atMobiles": [number] if number else [], "isAtAll": is_atall},
            }
            resp = await self._async_post(api, json_data)
            if resp.get("errcode") == 0 or resp.get("success"):
                results["success"].append(api)
            else:
                results["error"].append(api)
        return results

    async def send_to_wechat(
        self, url: str, title: str, content: str
    ) -> dict[str, Any]:
        results = {"success": [], "error": []}
        api_list = [u.strip() for u in url.replace("，", ",").split(",") if u.strip()]
        for api in api_list:
            json_data = {"title": title, "content": content}
            resp = await self._async_post(api, json_data)
            if resp.get("code") == 200 or resp.get("success"):
                results["success"].append(api)
            else:
                results["error"].append(api)
        return results

    async def send_to_telegram(
        self, chat_id: str, token: str, content: str, proxy: str | None = None
    ) -> dict[str, Any]:
        json_data = {"chat_id": chat_id, "text": content}
        url = f"https://api.telegram.org/bot{token}/sendMessage"
        resp = await self._async_post(url, json_data, proxy)
        return (
            {"success": [chat_id], "error": []}
            if "error" not in resp
            else {"success": [], "error": [chat_id]}
        )

    async def send_to_bark(
        self,
        api: str,
        title: str,
        content: str,
        level: str = "active",
        sound: str = "",
    ) -> dict[str, Any]:
        results = {"success": [], "error": []}
        api_list = [u.strip() for u in api.replace("，", ",").split(",") if u.strip()]
        for endpoint in api_list:
            json_data = {
                "title": title,
                "body": content,
                "level": level,
                "sound": sound,
            }
            resp = await self._async_post(endpoint, json_data)
            if resp.get("code") == 200 or resp.get("success"):
                results["success"].append(endpoint)
            else:
                results["error"].append(endpoint)
        return results

    async def send_to_ntfy(
        self,
        api: str,
        title: str,
        content: str,
        tags: str,
        action_url: str,
        email: str,
    ) -> dict[str, Any]:
        results = {"success": [], "error": []}
        api_list = [u.strip() for u in api.replace("，", ",").split(",") if u.strip()]
        tag_list = [
            u.strip() for u in tags.replace("，", ",").split(",") if u.strip()
        ] or ["tada"]
        for endpoint in api_list:
            json_data = {
                "title": title,
                "message": content,
                "tags": tag_list,
                "actions": [{"action": "view", "label": "open", "url": action_url}]
                if action_url
                else [],
                "email": email,
            }
            resp = await self._async_post(endpoint, json_data)
            if "error" not in resp:
                results["success"].append(endpoint)
            else:
                results["error"].append(endpoint)
        return results

    async def send_to_serverchan(
        self,
        sendkey: str,
        title: str,
        content: str,
        channel: str = "9",
        tags: str = "直播通知",
    ) -> dict[str, Any]:
        results = {"success": [], "error": []}
        sendkey_list = [
            u.strip() for u in sendkey.replace("，", ",").split(",") if u.strip()
        ]
        for key in sendkey_list:
            if key.startswith("sctp"):
                match = re.match(r"sctp(\d+)t", key)
                if not match:
                    results["error"].append(key)
                    continue
                num = match.group(1)
                url = f"https://{num}.push.ft07.com/send/{key}.send"
            else:
                url = f"https://sctapi.ftqq.com/{key}.send"
            json_data = {
                "title": title,
                "desp": content,
                "channel": int(channel or 9),
                "tags": tags,
            }
            resp = await self._async_post(url, json_data)
            if resp.get("code") == 0 or resp.get("success"):
                results["success"].append(key)
            else:
                results["error"].append(key)
        return results

    @staticmethod
    async def send_to_email(
        email_host: str,
        login_email: str,
        password: str,
        sender_email: str,
        sender_name: str,
        to_email: str,
        title: str,
        content: str,
    ) -> dict[str, Any]:
        receivers = [
            u.strip() for u in to_email.replace("，", ",").split(",") if u.strip()
        ]
        results = {"success": [], "error": []}
        try:
            message = MIMEMultipart()
            send_name = base64.b64encode(sender_name.encode("utf-8")).decode()
            message["From"] = f"=?UTF-8?B?{send_name}?= <{sender_email}>"
            message["Subject"] = Header(title, "utf-8")
            if len(receivers) == 1:
                message["To"] = receivers[0]

            message.attach(MIMEText(content, "plain", "utf-8"))
            smtp_obj = smtplib.SMTP_SSL(email_host, 465)
            smtp_obj.login(login_email, password)
            smtp_obj.sendmail(sender_email, receivers, message.as_string())
            results["success"] = receivers
        except smtplib.SMTPException:
            results["error"] = receivers
        return results
