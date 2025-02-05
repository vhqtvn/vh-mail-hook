import * as  age from 'age-encryption';

const DOMAIN = process.env.DOMAIN || 'https://mail-hook.libvh.dev/';
const API_KEY = process.env.API_KEY;

if (!API_KEY) {
    throw new Error('API_KEY is not set');
}

const MAILBOX_ID = process.env.MAILBOX_ID;

if (!MAILBOX_ID) {
    throw new Error('MAILBOX_ID is not set');
}

const PRIVATE_KEY = process.env.PRIVATE_KEY;

if (!PRIVATE_KEY) {
    throw new Error('PRIVATE_KEY is not set');
}

async function age_decrypt(encrypted_content) {
    const d = new age.Decrypter();
    d.addIdentity(PRIVATE_KEY);
    const encrypted_uint8_array = Buffer.from(encrypted_content, 'base64');
    const decrypted_uint8_array = await d.decrypt(encrypted_uint8_array);
    return Buffer.from(decrypted_uint8_array).toString('utf-8');
}

async function main() {
    const res = await fetch(`${DOMAIN.replace(/\/$/, '')}/api/v1/mailboxes/${MAILBOX_ID}/emails`, {
        headers: {
            'Authorization': `Bearer ${API_KEY}`
        }
    });
    const data = await res.json();
    // schema of data:
    // {
    //     success: boolean;
    //     data: {
    //         id: string;
    //         mailbox_id: string;
    //         encrypted_content: string;
    //         received_at: number;
    //         expires_at: number;
    //     }[];
    //     error: string | null;
    // }

    for(const email of data.data) {
        const decrypted_content = await age_decrypt(email.encrypted_content);
        console.log("================================================");
        console.log(decrypted_content);
        console.log("================================================");
    }
}

main();
