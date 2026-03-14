/**
 * Auth HTTP API
 *
 * Passwordless authentication with email codes and TOTP support.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace Auth;

// -- Models --

model RequestCodeInput {
  email: string;
}

model VerifyCodeInput {
  email: string;
  code: string;
}

model VerifyTotpInput {
  email: string;
  code: string;
}

model EnableTotpInput {
  secret: string;
  code: string;
}

model AuthToken {
  accessToken: string;
  tokenType: string;
  expiresIn: int64;
}

model TotpSetup {
  secret: string;
  otpauthUrl: string;
  qrCodeBase64: string;
}

model UserInfo {
  id: string;
  email: string;
  createdAt: string;
  lastLoginAt?: string;
  isAdmin: boolean;
}

model SubtokenInput {
  ttlSeconds?: int64;
}

model MessageResponse {
  message: string;
}

model ErrorResponse {
  error: string;
}

// -- Interfaces --

@route("/auth")
interface AuthService {
  @post
  @route("/request-code")
  requestCode(@body body: RequestCodeInput): {
    @statusCode statusCode: 200;
    @body body: MessageResponse;
  };

  @post
  @route("/verify")
  verifyCode(@body body: VerifyCodeInput): {
    @statusCode statusCode: 200;
    @body body: AuthToken;
  };

  @post
  @route("/verify-totp")
  verifyTotp(@body body: VerifyTotpInput): {
    @statusCode statusCode: 200;
    @body body: AuthToken;
  };

  @get
  @route("/me")
  getCurrentUser(): {
    @statusCode statusCode: 200;
    @body body: UserInfo;
  };

  @post
  @route("/totp/setup")
  setupTotp(): {
    @statusCode statusCode: 200;
    @body body: TotpSetup;
  };

  @post
  @route("/totp/enable")
  enableTotp(@body body: EnableTotpInput): {
    @statusCode statusCode: 200;
    @body body: MessageResponse;
  };

  @post
  @route("/totp/disable")
  disableTotp(): {
    @statusCode statusCode: 200;
    @body body: MessageResponse;
  };

  @post
  @route("/subtoken")
  createSubtoken(@body body: SubtokenInput): {
    @statusCode statusCode: 200;
    @body body: AuthToken;
  };
}
