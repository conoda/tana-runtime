// test/post.ts
import { Response } from "tana/net";
import { console } from "tana/core";
function Post(req, body) {
  console.log("POST request received:", req.path);
  console.log("Body:", body);
  return Response.json({
    message: "POST received!",
    echo: body,
    timestamp: Date.now()
  });
}
export {
  Post
};
