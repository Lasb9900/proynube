export type User={id:string;username:string;email:string;created_at:string};
export type Room={id:string;name:string;status:string;created_by:string;created_at:string};
export type Question={id:string;text:string;option_a:string;option_b:string;option_c:string;option_d:string;correct_option:string;anime_id:string};
export type ScoreEntry={user_id:string;points:number;rank:number};
